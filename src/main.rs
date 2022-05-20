mod config;
// mod near;
mod private_key;
mod last_block;
mod profit_estimation;
mod redis_wrapper;
mod transfer_event;
mod unlock_tokens;
mod transfer;
mod approve;
mod enqueue_tx;

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

    let storage = std::sync::Arc::new(std::sync::Mutex::new(last_block::Storage::new()));
    
    last_block::last_block_number_worker(
        settings.worker_interval,
        "https://rpc.testnet.near.org".to_string(),
        "client6.goerli.testnet".to_string(),
        storage.clone(),
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
        .launch()
        .await;
}
