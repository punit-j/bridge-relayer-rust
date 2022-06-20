mod approve;
mod async_redis_wrapper;
mod config;
mod ethereum;
mod last_block;
mod near;
mod pending_transactions_worker;
mod private_key;
mod profit_estimation;
mod transfer;
mod unlock_tokens;
mod utils;

#[macro_use]
extern crate rocket;

use crate::config::Settings;
use crate::ethereum::proof::Error;
use borsh::BorshSerialize;
use clap::Parser;
use near_crypto;
use near_sdk::AccountId;
use redis::{AsyncCommands, RedisResult, Value};
use rocket::State;
use secp256k1::ffi::PublicKey;
use serde_json::json;
use spectre_bridge_common::Proof;
use std::collections::HashMap;
use std::env;
use std::ops::Deref;
use std::os::linux::raw::stat;
use std::str::FromStr;
use std::thread::sleep;
use std::time::{Duration, SystemTime};
use tokio::task::JoinHandle;
use uint::rustc_hex::ToHex;
use url::quirks::hash;
use web3::signing::Key;
use web3::types::H256;

#[get("/health")]
fn health() -> String {
    "OK".to_string()
}

#[get("/transactions")]
async fn transactions(
    redis: &State<std::sync::Arc<std::sync::Mutex<async_redis_wrapper::AsyncRedisWrapper>>>,
) -> String {
    let mut r = redis.lock().unwrap().clone();
    json!(r.get_all().await).to_string()
}

#[post("/set_threshold", data = "<input>")]
fn set_threshold(input: String, settings: &State<std::sync::Arc<std::sync::Mutex<Settings>>>) {
    let json_data: serde_json::Value =
        serde_json::from_str(input.as_str()).expect("Cannot parse JSON request body");
    let new_threshold = json_data
        .get("profit_threshold")
        .unwrap()
        .as_u64()
        .expect("Cannot parse unsigned int");

    settings.lock().unwrap().set_threshold(new_threshold);
}

#[post("/set_allowed_tokens", data = "<input>")]
fn set_allowed_tokens(input: String, settings: &State<std::sync::Arc<std::sync::Mutex<Settings>>>) {
    let json_data: serde_json::Value =
        serde_json::from_str(input.as_str()).expect("Cannot parse JSON request body");

    let json_data_allowed_tokens = json_data.as_array().unwrap();

    let mut new_allowed_token_accounts: Vec<AccountId> = Vec::new();
    for val in json_data_allowed_tokens {
        let corrected_string = val.to_string().replace(&['\"'], "");
        new_allowed_token_accounts.push(AccountId::try_from(corrected_string).unwrap());
    }

    settings
        .lock()
        .unwrap()
        .set_allowed_tokens(new_allowed_token_accounts);
}

#[get("/profit")]
async fn profit(
    redis: &State<std::sync::Arc<std::sync::Mutex<async_redis_wrapper::AsyncRedisWrapper>>>,
) -> String {
    let mut r = redis.lock().unwrap().clone();
    json!(r.get_profit().await).to_string()
}

//
// Example of body request
//
// {
//     "6b175474e89094c44da98b954eedeac495271d0f.factory.bridge.near": "dai",
//      ...
// }
//
#[post("/set_mapped_tokens", data = "<input>")]
async fn set_mapped_tokens(
    input: String,
    settings: &State<std::sync::Arc<std::sync::Mutex<Settings>>>,
) {
    settings
        .lock()
        .unwrap()
        .clone()
        .set_mapped_tokens(serde_json::from_str(&input).expect("Failed to parse JSON request body"))
}

#[get("/get_mapped_tokens")]
async fn get_mapped_tokens(settings: &State<std::sync::Arc<std::sync::Mutex<Settings>>>) -> String {
    serde_json::to_string_pretty(&settings.lock().unwrap().clone().near_tokens_coin_id.mapping)
        .expect("Failed to parse to string mapped tokens")
}

//
// Example of body request
//
// {
//     "6b175474e89094c44da98b954eedeac495271d0f.factory.bridge.near": "dai",
//      ...
// }
//
#[post("/insert_mapped_tokens", data = "<input>")]
async fn insert_mapped_tokens(
    input: String,
    settings: &State<std::sync::Arc<std::sync::Mutex<Settings>>>,
) {
    settings.lock().unwrap().clone().insert_mapped_tokens(
        serde_json::from_str(&input).expect("Failed to parse JSON request body"),
    )
}

//
// Example of body request
//
// [
//     "6b175474e89094c44da98b954eedeac495271d0f.factory.bridge.near",
//     ...
// ]
//
#[post("/remove_mapped_tokens", data = "<input>")]
async fn remove_mapped_tokens(
    input: String,
    settings: &State<std::sync::Arc<std::sync::Mutex<Settings>>>,
) {
    settings.lock().unwrap().clone().remove_mapped_tokens(
        serde_json::from_str(&input).expect("Failed to parse JSON request body"),
    )
}

extern crate redis;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// config file
    #[clap(short, long)]
    config: String,

    /// eth secret key
    #[clap(long)]
    eth_secret: Option<String>,

    /// path to json file
    #[clap(long)]
    near_credentials: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let settings = match Settings::init(args.config) {
        Ok(settings) => std::sync::Arc::new(std::sync::Mutex::new(settings)),
        Err(msg) => panic!("{}", msg),
    };

    let async_redis = std::sync::Arc::new(std::sync::Mutex::new(
        async_redis_wrapper::AsyncRedisWrapper::connect(settings.clone()).await,
    ));

    let storage = std::sync::Arc::new(std::sync::Mutex::new(last_block::Storage::new()));

    // If args.eth_secret is valid then get key from it else from settings
    let eth_keypair = {
        if let Some(path) = args.eth_secret {
            secp256k1::SecretKey::from_str(&path.as_str())
        } else {
            secp256k1::SecretKey::from_str(&settings.lock().unwrap().eth.private_key)
        }
        .expect("Unable to get an Eth key")
    };
    let eth_keypair = web3::signing::SecretKeyRef::new(&eth_keypair);

    let eth_contract_address = settings.lock().unwrap().clone().eth.bridge_proxy_address;

    let eth_contract_abi = {
        let s = settings.lock().unwrap();
        eth_client::methods::get_contract_abi(
            &s.etherscan_api.endpoint_url.to_string(),
            s.eth.bridge_impl_address,
            &s.etherscan_api.api_key,
        )
        .await
    }
    .expect("Failed to get contract abi");

    let near_account = if let Some(path) = args.near_credentials {
        near_client::read_private_key::read_private_key_from_file(path.as_str())
    } else {
        near_client::read_private_key::read_private_key_from_file(
            settings.lock().unwrap().near.near_credentials_path.as_str(),
        )
    }
    .unwrap();

    let near_contract_address = settings.lock().unwrap().near.contract_address.clone();

    let near_worker = near::run_worker(near_contract_address, async_redis.clone(), {
        91966098 /*
                 let mut r = async_redis.lock().unwrap().clone();
                 if let Some(b) = r.option_get::<u64>(near::OPTION_START_BLOCK).await.unwrap() {b}
                 else {settings.lock().unwrap().near.near_lake_init_block}*/
    });

    let mut stream = async_redis_wrapper::subscribe::<String>(
        async_redis_wrapper::EVENTS.to_string(),
        async_redis.clone(),
    )
    .unwrap();
    let subscriber = {
        let settings = settings.clone();
        let rpc_url = settings.lock().unwrap().eth.rpc_url.clone();
        let eth_keypair = eth_keypair.clone();
        let redis = async_redis.clone();
        let eth_contract_abi = &eth_contract_abi;
        async move {
            while let Some(msg) = stream.recv().await {
                if let Ok(event) =
                    serde_json::from_str::<spectre_bridge_common::Event>(msg.as_str())
                {
                    println!("event {:?}", event);

                    match event {
                        spectre_bridge_common::Event::SpectreBridgeTransferEvent {
                            nonce,
                            chain_id,
                            valid_till,
                            transfer,
                            fee,
                            recipient,
                        } => {
                            let near_tokens_coin_id = &settings.lock().unwrap().near_tokens_coin_id;

                            let tx_hash = transfer::execute_transfer(
                                &eth_keypair,
                                spectre_bridge_common::Event::SpectreBridgeTransferEvent {
                                    nonce,
                                    chain_id,
                                    valid_till,
                                    transfer,
                                    fee,
                                    recipient,
                                },
                                eth_contract_abi.as_bytes(),
                                rpc_url.as_str(),
                                eth_contract_address.clone(),
                                0.0,
                                near_tokens_coin_id,
                            )
                            .await;

                            match tx_hash {
                                Some(hash) => {
                                    let d = crate::async_redis_wrapper::PendingTransactionData {
                                        timestamp: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs(),
                                        nonce: u128::from(nonce),
                                    };

                                    let res: redis::RedisResult<()> = redis
                                        .lock()
                                        .unwrap()
                                        .connection
                                        .hset(
                                            async_redis_wrapper::PENDING_TRANSACTIONS,
                                            hash.as_bytes().to_hex::<String>(),
                                            serde_json::to_string(&d).unwrap(),
                                        )
                                        .await;
                                    if let Err(e) = res {
                                        eprintln!("Unable to store pending transaction: {}", e);
                                    }
                                }
                                None => (),
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    };

    let pending_transactions_worker = {
        let s = settings.lock().unwrap();
        pending_transactions_worker::run(
            s.eth.rpc_url.clone(),
            eth_contract_address,
            eth_contract_abi.clone(),
            &eth_keypair,
            async_redis.lock().unwrap().clone(),
            if s.eth.pending_transaction_poll_delay_sec > 0 {
                s.eth.pending_transaction_poll_delay_sec as u64
            } else {
                5
            },
        )
    };

    let last_block_number_worker =
        last_block::last_block_number_worker(settings.clone(), storage.clone());

    let unlock_tokens_worker = unlock_tokens::unlock_tokens_worker(
        near_account.clone(),
        300_000_000_000_000,
        settings.clone(),
        storage.clone(),
        async_redis.clone(),
    );

    let rocket = rocket::build()
        .mount(
            "/v1",
            routes![
                health,
                transactions,
                set_threshold,
                set_allowed_tokens,
                profit,
                set_mapped_tokens,
                get_mapped_tokens,
                insert_mapped_tokens,
                remove_mapped_tokens,
            ],
        )
        .manage(settings)
        .manage(storage)
        .manage(async_redis);

    tokio::join!(
        near_worker,
        subscriber,
        pending_transactions_worker,
        last_block_number_worker,
        unlock_tokens_worker,
        rocket.launch()
    );
}

#[cfg(test)]
pub mod tests {
    const NEAR_RPC_ENDPOINT_URL: &str = "https://rpc.testnet.near.org";
    const ETH_RPC_ENDPOINT_URL: &str =
        "https://goerli.infura.io/v3/ba5fd6c86e5c4e8c9b36f3f5b4013f7a";
    const ETHERSCAN_RPC_ENDPOINT_URL: &str = "https://api-goerli.etherscan.io";

    #[tokio::test]
    async fn near_rpc_status() {
        let client = near_jsonrpc_client::JsonRpcClient::connect(NEAR_RPC_ENDPOINT_URL);
        let status = client
            .call(near_jsonrpc_client::methods::status::RpcStatusRequest)
            .await;
        assert!(
            matches!(
                status,
                Ok(near_jsonrpc_client::methods::status::RpcStatusResponse { .. })
            ),
            "expected an Ok(RpcStatusResponse), found [{:?}]",
            status
        );
    }

    #[tokio::test]
    pub async fn eth_rpc_status() {
        let transport = web3::transports::Http::new(ETH_RPC_ENDPOINT_URL);
        assert!(transport.is_ok());
    }

    #[tokio::test]
    pub async fn etherscan_rpc_status() {
        let status = reqwest::get(ETHERSCAN_RPC_ENDPOINT_URL).await;
        assert!(status.is_ok());
        assert_eq!(reqwest::StatusCode::OK, status.unwrap().status());
    }
}
