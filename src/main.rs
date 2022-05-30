mod config;
// mod near;
mod approve;
mod async_redis_wrapper;
mod enqueue_tx;
mod last_block;
mod private_key;
mod profit_estimation;
mod redis_wrapper;
mod transfer;
mod transfer_event;
mod unlock_tokens;

#[macro_use]
extern crate rocket;

use crate::config::Settings;
use crate::redis_wrapper::RedisWrapper;
use near_sdk::AccountId;
use rocket::State;
use serde_json::json;
use std::env;

#[get("/health")]
fn health() -> String {
    "OK".to_string()
}

#[get("/transactions")]
fn transactions(redis: &State<RedisWrapper>) -> String {
    json!(redis.get_all()).to_string()
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
fn profit(redis: &State<RedisWrapper>) -> String {
    json!(redis.get_profit()).to_string()
}

extern crate redis;

#[rocket::main]
async fn main() {
    // Reading arguments that was given to binary
    let args: Vec<String> = env::args().collect();
    let config_file_path = args.get(1).unwrap().to_string();

    let settings = Settings::init(config_file_path);

    let redis = RedisWrapper::connect(settings.redis_setting.clone());

    let async_redis = std::sync::Arc::new(std::sync::Mutex::new(
        async_redis_wrapper::AsyncRedisWrapper::connect(settings.redis_setting.clone()).await,
    ));

    let storage = std::sync::Arc::new(std::sync::Mutex::new(last_block::Storage::new()));

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

    let _res = rocket::build()
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
        .manage(redis)
        .manage(storage)
        .manage(async_redis)
        .launch()
        .await;
}
