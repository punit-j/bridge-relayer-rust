mod approve;
mod async_redis_wrapper;
mod config;
mod enqueue_tx;
mod last_block;
mod near;
mod private_key;
mod profit_estimation;
mod transfer;
mod transfer_event;
mod unlock_tokens;
mod redis_subscriber;
mod redis_publisher;
mod message;
mod message_handler;

#[macro_use]
extern crate rocket;

use crate::config::Settings;
use near_sdk::AccountId;
use rocket::State;
use serde_json::json;
use std::env;
use std::thread::sleep;
use std::time::Duration;

#[get("/health")]
fn health() -> String {
    "OK".to_string()
}

#[get("/transactions")]
async fn transactions(redis: &State<std::sync::Arc<std::sync::Mutex<async_redis_wrapper::AsyncRedisWrapper>>>) -> String {
    let mut r = redis.lock().unwrap().clone();
    json!(r.get_all().await).to_string()
}

#[post("/set_threshold", data = "<input>")]
fn set_threshold(input: String, settings: &State<Settings>) {
    let json_data: serde_json::Value =
        serde_json::from_str(input.as_str()).expect("Cannot parse JSON request body");
    let new_threshold = json_data
        .get("profit_threshold")
        .unwrap()
        .as_u64()
        .expect("Cannot parse unsigned int");

    settings.set_threshold(new_threshold);
}

#[post("/set_allowed_tokens", data = "<input>")]
fn set_allowed_tokens(input: String, settings: &State<Settings>) {
    let json_data: serde_json::Value =
        serde_json::from_str(input.as_str()).expect("Cannot parse JSON request body");

    let json_data_allowed_tokens = json_data.as_array().unwrap();

    let mut new_allowed_token_accounts: Vec<AccountId> = Vec::new();
    for val in json_data_allowed_tokens {
        let corrected_string = val.to_string().replace(&['\"'], "");
        new_allowed_token_accounts.push(AccountId::try_from(corrected_string).unwrap());
    }

    settings.set_allowed_tokens(new_allowed_token_accounts);
}

#[get("/profit")]
async fn profit(redis: &State<std::sync::Arc<std::sync::Mutex<async_redis_wrapper::AsyncRedisWrapper>>>) -> String {
    let mut r = redis.lock().unwrap().clone();
    json!(r.get_profit().await).to_string()
}

extern crate redis;

#[rocket::main]
async fn main() {
    // Reading arguments that was given to binary
    let args: Vec<String> = env::args().collect();

    let config_file_path = args.get(1).unwrap().to_string();

    let settings = Settings::init(config_file_path);

    let async_redis = std::sync::Arc::new(std::sync::Mutex::new(
        async_redis_wrapper::AsyncRedisWrapper::connect(settings.redis_setting.clone()).await,
    ));

    let storage = std::sync::Arc::new(std::sync::Mutex::new(last_block::Storage::new()));

    let near_worker = near::run_worker(&settings.near_settings.contract_address,
                             async_redis.clone(),
                             {
                                 let mut r = async_redis.lock().unwrap().clone();
                                 if let Some(b) = r.option_get::<u64>(near::OPTION_START_BLOCK).await.unwrap() {b}
                                 else {settings.near_settings.near_lake_init_block}
                             }
    );

    let subscriber = redis_subscriber::subscribe(async_redis_wrapper::EVENTS.to_string(), async_redis.clone());

    tokio::join!(near_worker, subscriber);  // tests...

    last_block::last_block_number_worker(
        "https://rpc.testnet.near.org".to_string(),
        "arseniyrest.testnet".to_string(),
        near_client::read_private_key::read_private_key_from_file(
            "/home/arseniyk/.near-credentials/testnet/arseniyrest.testnet.json",
        ),
        "client6.goerli.testnet".to_string(),
        300_000_000_000_000,
        15,
        storage.clone(),
    )
        .await;

    unlock_tokens::unlock_tokens_worker(
        "https://rpc.testnet.near.org".to_string(),
        "arseniyrest.testnet".to_string(),
        near_client::read_private_key::read_private_key_from_file(
            "/home/arseniyk/.near-credentials/testnet/arseniyrest.testnet.json",
        ),
        "client6.goerli.testnet".to_string(),
        300_000_000_000_000,
        5,
        2,
        storage.clone(),
        async_redis.clone(),
    )
        .await;

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
        .launch()
        .await;
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
