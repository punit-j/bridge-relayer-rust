mod approve;
mod async_redis_wrapper;
mod config;
mod last_block;
mod near;
mod private_key;
mod profit_estimation;
mod transfer;
mod unlock_tokens;
mod message;
mod message_handler;
mod utils;

#[macro_use]
extern crate rocket;

use crate::config::Settings;
use near_sdk::AccountId;
use rocket::State;
use serde_json::json;
use std::env;
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;
use borsh::BorshSerialize;
use secp256k1::ffi::PublicKey;
use clap::Parser;

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
}

#[rocket::main]
async fn main() {
    let args = Args::parse();

    let settings = match Settings::init(args.config) {
        Ok(s) => std::sync::Arc::new(std::sync::Mutex::new(s)),
        Err(msg) => panic!("{}", msg),
    };

    // If args.eth_secret is valid then get key from it else from settings
    let eth_keypair = {
        let secp = secp256k1::Secp256k1::new();
        if let Some(path) = args.eth_secret {
            secp256k1::KeyPair::from_seckey_str(&secp,&path.as_str())
        } else {
            secp256k1::KeyPair::from_seckey_str(&secp,&settings.lock().unwrap().eth.private_key)
        }.expect("Unable to get the Eth key")
    };

    let eth_contract_address = settings.lock().unwrap().eth.contract_address.clone();

    let eth_contract_abi = eth_client::methods::get_contract_abi(
        "https://api-rinkeby.etherscan.io",
        eth_contract_address.as_str(),
        "",
    )
        .await
        .expect("Failed to get contract abi");

    let async_redis = std::sync::Arc::new(std::sync::Mutex::new(
        async_redis_wrapper::AsyncRedisWrapper::connect(settings.clone()).await,
    ));

    let storage = std::sync::Arc::new(std::sync::Mutex::new(last_block::Storage::new()));

    let near_contract_address = settings.lock().unwrap().near.contract_address.clone();
    let near_lake_init_block = settings.lock().unwrap().near.near_lake_init_block;
    let near_worker = near::run_worker(near_contract_address,
                                       async_redis.clone(),
                                       {
                                           /*let mut r = async_redis.lock().unwrap().clone();
                                           if let Some(b) = r.option_get::<u64>(near::OPTION_START_BLOCK).await.unwrap() {b}
                                           else {settings.near_settings.near_lake_init_block}*/
                                           near_lake_init_block
                                       }
    );

    let mut stream = async_redis_wrapper::subscribe::<String>(async_redis_wrapper::EVENTS.to_string(), async_redis.clone()).unwrap();
    let subscriber = {
        let settings = settings.clone();
        let rpc_url = settings.lock().unwrap().eth.rpc_url.clone();
        async move {
            while let Some(msg) = stream.recv().await {
                if let Ok(event) = serde_json::from_str::<spectre_bridge_common::Event>(msg.as_str()) {
                    println!("event {:?}", event);

                    let token_addr = web3::types::Address::from_str("b2d75C5a142A68BDA438e6a318C7FBB2242f9693").unwrap();

                    match event {
                        spectre_bridge_common::Event::SpectreBridgeTransferEvent { nonce, chain_id, valid_till, mut transfer, fee, recipient } => {

                            // TODO: Haddcoded token
                            transfer.token_eth = spectre_bridge_common::EthAddress::from(token_addr);

                            transfer::execute_transfer(eth_keypair.public_key().to_string().as_str(), eth_keypair.display_secret().to_string().as_str(), // TODO: don't sure
                                                       spectre_bridge_common::Event::SpectreBridgeTransferEvent { nonce, chain_id, valid_till, transfer, fee, recipient },
                                                       &eth_contract_abi.as_bytes(), rpc_url.as_str(), eth_contract_address.as_str(), 0.0);
                        },
                        _ => {}
                    }
                }
            }
        }
    };

    let last_block_number_worker = last_block::last_block_number_worker(settings.clone(), storage.clone());
    /*
        let unlock_tokens_worker = unlock_tokens::unlock_tokens_worker(
            "arseniyrest.testnet".to_string(),
            near_client::read_private_key::read_private_key_from_file(
                "/home/arseniyk/.near-credentials/testnet/arseniyrest.testnet.json",
            ),
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
                                   profit
                               ],
            )
            .manage(settings)
            .manage(storage)
            .manage(async_redis)
            .launch();
    */
    tokio::join!(near_worker, subscriber, /*rocket, unlock_tokens_worker*/); // tests...
}

#[cfg(test)]
pub mod tests {
    /*
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
        }*/
}
