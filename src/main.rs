mod config;
mod redis_wrapper;
mod transfer_event;
mod near;
mod profit_estimation;
mod unlock_tokens;

#[macro_use]
extern crate rocket;

use crate::config::Settings;
use crate::redis_wrapper::RedisWrapper;
use near_sdk::AccountId;
use rocket::State;
use serde_json::json;
use std::env;

use http_client::HttpClient;
use http_types::{Method, Request};

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

#[post("/vault_secret", data = "<input>")]
async fn vault_secret(input: String, settings: &State<Settings>) -> String
{
    let json_data: serde_json::Value =
        serde_json::from_str(input.as_str()).expect("Cannot parse JSON request body");

    let token = String::from(json_data.get("token").unwrap().as_str().unwrap());

    let client = http_client::h1::H1Client::new();
    let mut addr = settings.vault_addr.to_string();
    addr.push_str("key2"); // Example, but it can be any const str

    let mut req = Request::new(Method::Get, addr.as_str());
    req.insert_header("X-Vault-Token", &token);

    let res = client.send(req).await.unwrap().body_string().await.unwrap();
    json!(res).to_string()
}

extern crate redis;

#[rocket::main]
async fn main() {
    // Reading arguments that was given to binary
    let args: Vec<String> = env::args().collect();
    let config_file_path = args.get(1).unwrap().to_string();

    let settings = Settings::init(config_file_path);
    let redis = RedisWrapper::connect(settings.redis_setting.clone());

    let _res = rocket::build()
        .mount(
            "/v1",
            routes![health, transactions, set_threshold, set_allowed_tokens, profit, vault_secret],
        )
        .manage(settings)
        .manage(redis)
        .launch()
        .await;
}
