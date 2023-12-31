use borsh::{BorshDeserialize, BorshSerialize};
use eth_client::methods::get_contract_abi;
use fast_bridge_common::{EthAddress, TransferDataEthereum, TransferDataNear};
use fast_bridge_service_lib::async_redis_wrapper::{self, AsyncRedisWrapper};
use fast_bridge_service_lib::async_redis_wrapper::{
    subscribe, EVENTS, NEW_EVENTS, PENDING_TRANSACTIONS, TRANSACTIONS,
};

use fast_bridge_service_lib::config::{
    Decimals, NearNetwork, NearTokenInfo, SafeSettings, Settings,
};
use fast_bridge_service_lib::last_block::{last_block_number_worker, SafeStorage, Storage};
use fast_bridge_service_lib::logs::init_logger;
use fast_bridge_service_lib::unlock_tokens::unlock_tokens_worker;
use near_client::read_private_key::read_private_key_from_file;
use near_crypto::InMemorySigner;
use near_primitives::views::FinalExecutionStatus;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::timeout;
use url::Url;

const ETH_CONTRACT_PROXY_ADDRESS: &str = "B8Dc44944f2C3149d6A7477Eb1E61C454AFE2e5e";
const ETH_CONTRACT_IMPLEMENTATION_ADDRESS: &str = "878b8ADBbDd5D9ca52789F463e8cF27094b534c8";
const ETH_TOKEN_ADDRESS: &str = "b2d75C5a142A68BDA438e6a318C7FBB2242f9693";
const NEAR_CONTRACT_ADDRESS: &str = "fast-bridge3.olga24912_3.testnet";
const NEAR_TOKEN_ADDRESS: &str = "token.olga24912_3.testnet";

const ONE_MINUTE_SEC: u64 = 60;
const TRANSFER_TOKEN_AMOUNT: u128 = 1000000;
const FEE_TOKEN_AMOUNT: u128 = 20000000;
const ONE_HOUR_IN_SECS: u64 = 60 * 60;

#[tokio::test]
async fn main_integration_test() {
    init_logger();

    let near_relay_signer = get_near_signer();
    mint_near_tokens(near_relay_signer.clone()).await;
    increase_fast_bridge_token_balance(near_relay_signer.clone()).await;
    let block_hash = init_token_transfer(near_relay_signer.clone()).await;

    let init_block = get_block_height(
        near_workspaces::CryptoHash::try_from(block_hash.as_bytes().as_slice()).unwrap(),
    )
    .await;

    let settings = get_settings();
    let settings = std::sync::Arc::new(tokio::sync::Mutex::new(settings));
    let redis = fast_bridge_service_lib::async_redis_wrapper::AsyncRedisWrapper::connect(
        &settings.lock().await.redis,
    )
    .await;
    remove_all(redis.clone(), PENDING_TRANSACTIONS).await;
    remove_all(redis.clone(), TRANSACTIONS).await;
    remove_all(redis.clone(), NEW_EVENTS).await;

    detect_new_near_event(redis.clone(), init_block, 10).await;

    let relay_eth_key = std::sync::Arc::new(
        secp256k1::SecretKey::from_str(
            &settings.lock().await.eth.private_key.clone().unwrap()[..64],
        )
        .unwrap(),
    );
    let eth_contract_abi = std::sync::Arc::new(get_eth_erc20_fast_bridge_contract_abi().await);
    let eth_contract_address = std::sync::Arc::new(eth_addr(ETH_CONTRACT_PROXY_ADDRESS));

    increase_allowance().await;
    tokio::time::sleep(Duration::from_secs(ONE_MINUTE_SEC)).await;
    mint_eth_tokens().await;
    tokio::time::sleep(Duration::from_secs(ONE_MINUTE_SEC)).await;

    process_events(
        settings.clone(),
        relay_eth_key.clone(),
        redis.clone(),
        eth_contract_abi.clone(),
        eth_contract_address.clone(),
        near_relay_signer.account_id.to_string(),
    )
    .await;

    handle_pending_transaction(settings.clone(), redis.clone()).await;

    let storage = std::sync::Arc::new(tokio::sync::Mutex::new(Storage::new()));
    let _last_block_worker = last_block_number_worker(settings.clone(), storage.clone()).await;
    wait_correct_last_block_number(storage.clone(), redis.clone()).await;

    let init_block = get_finality_block_height().await;

    let worker = unlock_tokens_worker(
        near_relay_signer,
        230_000_000_000_000u64,
        settings.clone(),
        storage.clone(),
        redis.clone(),
    );
    let timeout_duration = std::time::Duration::from_secs(60);
    let _result = timeout(timeout_duration, worker).await;

    let stream = subscribe::<String>(EVENTS.to_string(), redis.clone()).unwrap();
    detect_new_near_event(redis.clone(), init_block, ONE_MINUTE_SEC).await;
    check_unlock_event(stream).await;
}

async fn wait_correct_last_block_number(storage: SafeStorage, mut redis: AsyncRedisWrapper) {
    let tx_hashes_queue = redis.get_tx_hashes().await.unwrap();
    let tx_hash = tx_hashes_queue[0].clone();
    let tx_block = redis.get_tx_data(tx_hash.clone()).await.unwrap().block;

    let mut eth_last_block_number_on_near = 0;

    const MAX_ITERATION_NUMBER: u64 = 100;
    let mut iter_number = 0;

    while eth_last_block_number_on_near < tx_block && iter_number < MAX_ITERATION_NUMBER {
        iter_number += 1;
        tokio::time::sleep(Duration::from_secs(30)).await;

        eth_last_block_number_on_near = storage.lock().await.clone().eth_last_block_number_on_near;
        tracing::info!(
            "Current last block: {};, tx_block: {}",
            eth_last_block_number_on_near,
            tx_block
        );
    }

    if iter_number == MAX_ITERATION_NUMBER {
        panic!("Last block number wasn't updated for a while. Please, check that EthOnNearClient works properly.");
    }
}

fn eth_addr(addr_str: &str) -> web3::types::Address {
    web3::types::Address::from_str(addr_str).unwrap()
}

fn near_addr(addr_str: &str) -> near_sdk::AccountId {
    addr_str.parse().unwrap()
}

fn get_recipient_eth_addr() -> web3::types::Address {
    eth_addr("2a23E0Fa3Afe77AFf5dc6c6a007E3A10c1450633")
}

fn abspath(p: &str) -> Option<String> {
    shellexpand::full(p)
        .ok()
        .and_then(|x| Path::new(OsStr::new(x.as_ref())).canonicalize().ok())
        .and_then(|p| p.into_os_string().into_string().ok())
}

fn get_near_signer() -> InMemorySigner {
    let path = "~/.near-credentials/testnet/fastbridge.testnet.json";
    let absolute = abspath(path).unwrap();
    read_private_key_from_file(&absolute).unwrap()
}

fn get_near_endpoint_url() -> url::Url {
    url::Url::parse("https://rpc.testnet.near.org").unwrap()
}

pub fn get_valid_till() -> u64 {
    (std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        + Duration::from_secs(3 * ONE_HOUR_IN_SECS).as_nanos()) as u64
}

fn get_eth_rpc_url() -> Url {
    let api_key_string = env::var("FAST_BRIDGE_INFURA_PROJECT_ID").unwrap();
    url::Url::parse(&format!("https://goerli.infura.io/v3/{}", &api_key_string)).unwrap()
}

fn get_rb_index_path_str() -> String {
    let path_to_rainbow_bridge_rep_str = env::var("PATH_TO_RAINBOW_BRIDGE_REP").unwrap();
    let path_to_rainbow_bridge_rep = Path::new(&path_to_rainbow_bridge_rep_str);
    let rb_index_path = path_to_rainbow_bridge_rep.join("cli/index.js");
    rb_index_path.to_str().unwrap().to_string()
}

fn get_settings() -> Settings {
    let config_path = "config.json.example";
    let mut settings = Settings::init(config_path.to_string()).unwrap();
    settings.eth.rainbow_bridge_index_js_path = get_rb_index_path_str();
    settings.near_tokens_whitelist.mapping.insert(
        near_addr(NEAR_TOKEN_ADDRESS),
        NearTokenInfo {
            exchange_id: "wrapped-near".to_owned(),
            fixed_fee: 0.into(),
            percent_fee: 0.0,
            decimals: Decimals::try_from(6).unwrap(),
            eth_address: ETH_TOKEN_ADDRESS.parse().unwrap(),
            max_transfer_amount: None,
        },
    );

    settings.unlock_tokens_worker.contract_account_id = NEAR_CONTRACT_ADDRESS.to_string();
    settings.unlock_tokens_worker.blocks_for_tx_finalization = 0;
    settings
}

pub async fn get_eth_erc20_fast_bridge_contract_abi() -> String {
    let etherscan_endpoint_url = "https://api-goerli.etherscan.io";
    let eth_bridge_impl_address = eth_addr(ETH_CONTRACT_IMPLEMENTATION_ADDRESS);
    let etherscan_api_key = env::var("FAST_BRIDGE_ETHERSCAN_API_KEY").unwrap();
    get_contract_abi(
        etherscan_endpoint_url,
        eth_bridge_impl_address,
        &etherscan_api_key,
    )
    .await
    .unwrap()
}

async fn remove_all(mut redis: crate::async_redis_wrapper::AsyncRedisWrapper, key: &str) {
    let mut iter: redis::AsyncIter<(String, String)> = redis.connection.hscan(key).await.unwrap();

    let mut keys = vec![];
    while let Some(pair) = iter.next_item().await {
        keys.push(pair.0);
    }
    for hash in keys {
        let _res: () = redis.connection.hdel(key, hash).await.unwrap();
    }
}

async fn check_unlock_event(mut stream: tokio::sync::mpsc::Receiver<String>) {
    let timeout_duration = std::time::Duration::from_secs(10);

    let recv_event = serde_json::from_str::<fast_bridge_common::Event>(
        &timeout(timeout_duration, stream.recv())
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();

    if let fast_bridge_common::Event::FastBridgeLpUnlockEvent { .. } = recv_event {
        println!("Unlock event: {:?}", recv_event);
    } else {
        panic!("Don't get unlock event!")
    }
}

async fn mint_near_tokens(signer: InMemorySigner) {
    let server_addr = get_near_endpoint_url();
    let contract_account_id = NEAR_TOKEN_ADDRESS.to_string();
    let method_name = "mint".to_string();
    let args = json!({"account_id": signer.account_id, "amount": format!("{}", TRANSFER_TOKEN_AMOUNT + FEE_TOKEN_AMOUNT)});
    let response = near_client::methods::change(
        server_addr,
        signer,
        contract_account_id,
        method_name,
        args,
        4_000_000_000_000,
        0,
    )
    .await
    .unwrap();

    if let FinalExecutionStatus::SuccessValue(_) = response.status {
        println!("Tokens on NEAR MINT successfully");
    } else {
        panic!("Mint tokens on NEAR FAIL");
    }
}

async fn increase_fast_bridge_token_balance(signer: InMemorySigner) {
    let server_addr = get_near_endpoint_url();
    let contract_account_id = NEAR_TOKEN_ADDRESS.to_string();
    let method_name = "ft_transfer_call".to_string();
    let args = json!({"receiver_id": near_addr(NEAR_CONTRACT_ADDRESS), "amount": format!("{}", TRANSFER_TOKEN_AMOUNT + FEE_TOKEN_AMOUNT), "msg": ""});
    let response = near_client::methods::change(
        server_addr,
        signer,
        contract_account_id,
        method_name,
        args,
        300_000_000_000_000,
        1,
    )
    .await
    .unwrap();

    if let FinalExecutionStatus::SuccessValue(_) = response.status {
        println!("Tokens on NEAR moved to the Bridge Contract successfully");
    } else {
        panic!(
            "Moving tokens to Bridge Contract on NEAR FAIL {:?}",
            response
        );
    }
}

#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct TransferMessage {
    valid_till: u64,
    transfer: TransferDataEthereum,
    fee: TransferDataNear,
    recipient: EthAddress,
    valid_till_block_height: Option<u64>,
}

async fn init_token_transfer(signer: InMemorySigner) -> near_primitives::hash::CryptoHash {
    let server_addr = get_near_endpoint_url();
    let contract_account_id = NEAR_CONTRACT_ADDRESS.to_string();
    let method_name = "init_transfer".to_string();

    let transfer_message = TransferMessage {
        valid_till: get_valid_till(),
        transfer: TransferDataEthereum {
            token_near: near_addr(NEAR_TOKEN_ADDRESS),
            token_eth: EthAddress(eth_addr(ETH_TOKEN_ADDRESS).into()),
            amount: near_sdk::json_types::U128(TRANSFER_TOKEN_AMOUNT),
        },
        fee: TransferDataNear {
            token: near_addr(NEAR_TOKEN_ADDRESS),
            amount: near_sdk::json_types::U128(FEE_TOKEN_AMOUNT),
        },
        recipient: EthAddress(get_recipient_eth_addr().into()),
        valid_till_block_height: None,
    };

    let args = json!({ "msg": near_sdk::base64::encode(transfer_message.try_to_vec().unwrap()) });
    let response = near_client::methods::change(
        server_addr,
        signer,
        contract_account_id,
        method_name,
        args,
        300_000_000_000_000,
        0,
    )
    .await
    .unwrap();

    if let FinalExecutionStatus::SuccessValue(_) = response.status {
        println!("Tokens transfer init successfully");
    } else {
        panic!("Token transfer init FAIL. Response: {:?}", response);
    }

    response.receipts_outcome.last().unwrap().block_hash
}

async fn get_block_height(block_hash: near_workspaces::CryptoHash) -> u64 {
    let worker = near_workspaces::testnet().await.unwrap();
    let block = worker.view_block().block_hash(block_hash).await.unwrap();

    block.height()
}

async fn get_finality_block_height() -> u64 {
    let worker = near_workspaces::testnet().await.unwrap();
    let block = worker.view_block().await.unwrap();

    block.height()
}

async fn detect_new_near_event(redis: AsyncRedisWrapper, init_block: u64, wait_time_sec: u64) {
    let contract_address =
        near_lake_framework::near_indexer_primitives::types::AccountId::from_str(
            NEAR_CONTRACT_ADDRESS,
        )
        .unwrap();

    let worker = fast_bridge_service_lib::near_events_tracker::run_worker(
        contract_address,
        redis,
        init_block,
        NearNetwork::Testnet,
    );
    let timeout_duration = std::time::Duration::from_secs(wait_time_sec);
    let _result = timeout(timeout_duration, worker).await;
}

async fn process_events(
    settings: SafeSettings,
    eth_keypair: std::sync::Arc<secp256k1::SecretKey>,
    mut redis: AsyncRedisWrapper,
    eth_contract_abi: std::sync::Arc<String>,
    eth_contract_address: std::sync::Arc<web3::types::Address>,
    near_relay_account_id: String,
) {
    let worker = fast_bridge_service_lib::near_event_processor::process_near_events_worker(
        settings.clone(),
        eth_keypair.clone(),
        redis.clone(),
        eth_contract_abi.clone(),
        eth_contract_address.clone(),
        near_relay_account_id,
    );
    let timeout_duration = std::time::Duration::from_secs(120);
    let _result = timeout(timeout_duration, worker).await;

    let pending_transactions: Vec<String> =
        redis.connection.hkeys(PENDING_TRANSACTIONS).await.unwrap();
    assert_eq!(pending_transactions.len(), 1);
}

async fn handle_pending_transaction(settings: SafeSettings, redis: AsyncRedisWrapper) {
    let locked_settings = settings.lock().await.clone();
    let worker = fast_bridge_service_lib::pending_transactions_worker::run(
        locked_settings.eth.rpc_url,
        locked_settings.eth.rainbow_bridge_index_js_path.clone(),
        redis.clone(),
        locked_settings.rpc_timeout_secs,
    );

    let timeout_duration = std::time::Duration::from_secs(30);
    let _result = timeout(timeout_duration, worker).await;

    let pending_transactions: Vec<String> = redis
        .clone()
        .connection
        .hkeys(PENDING_TRANSACTIONS)
        .await
        .unwrap();
    assert_eq!(pending_transactions.len(), 0);

    let transactions: Vec<String> = redis.clone().connection.hkeys(TRANSACTIONS).await.unwrap();
    assert_eq!(transactions.len(), 1);
}

async fn mint_eth_tokens() {
    let eth1_endpoint = get_eth_rpc_url();

    let token = eth_addr(ETH_TOKEN_ADDRESS);

    let etherscan_endpoint_url = "https://api-goerli.etherscan.io";
    let etherscan_api_key = env::var("FAST_BRIDGE_ETHERSCAN_API_KEY").unwrap();
    let contract_abi = get_contract_abi(etherscan_endpoint_url, token, &etherscan_api_key)
        .await
        .unwrap();

    let method_name = "mint";
    let amount = web3::types::U256::from(TRANSFER_TOKEN_AMOUNT + FEE_TOKEN_AMOUNT);

    let priv_key =
        secp256k1::SecretKey::from_str(&(env::var("FAST_BRIDGE_ETH_PRIVATE_KEY").unwrap())[..64])
            .unwrap();

    let res = eth_client::methods::change(
        eth1_endpoint,
        token,
        contract_abi.as_bytes(),
        &method_name,
        amount,
        &priv_key,
        true,
        None,
        None,
        None,
        30,
    )
    .await
    .unwrap();

    println!("transaction hash: {:?}", res);
}

async fn increase_allowance() {
    let eth1_endpoint = get_eth_rpc_url();

    let token = eth_addr(ETH_TOKEN_ADDRESS);

    let etherscan_endpoint_url = "https://api-goerli.etherscan.io";
    let etherscan_api_key = env::var("FAST_BRIDGE_ETHERSCAN_API_KEY").unwrap();
    let contract_abi = get_contract_abi(etherscan_endpoint_url, token, &etherscan_api_key)
        .await
        .unwrap();

    let method_name = "increaseAllowance";
    let spender = eth_addr(ETH_CONTRACT_PROXY_ADDRESS);
    let amount = web3::types::U256::from(TRANSFER_TOKEN_AMOUNT + FEE_TOKEN_AMOUNT);
    let method_args = (spender, amount);

    let priv_key =
        secp256k1::SecretKey::from_str(&(env::var("FAST_BRIDGE_ETH_PRIVATE_KEY").unwrap())[..64])
            .unwrap();

    let res = eth_client::methods::change(
        eth1_endpoint,
        token,
        contract_abi.as_bytes(),
        &method_name,
        method_args,
        &priv_key,
        true,
        None,
        None,
        None,
        30,
    )
    .await
    .unwrap();

    println!("transaction hash: {:?}", res);
}
