use crate::{async_redis_wrapper, Settings};
use near_sdk::AccountId;
use rocket::{get, post, State};
use serde_json::json;

#[get("/health")]
pub fn health() -> String {
    "OK".to_string()
}

#[get("/transactions")]
pub async fn transactions(
    redis: &State<std::sync::Arc<std::sync::Mutex<async_redis_wrapper::AsyncRedisWrapper>>>,
) -> String {
    let mut r = redis.lock().unwrap().clone();
    json!(r.get_all().await).to_string()
}

#[post("/set_threshold", data = "<input>")]
pub fn set_threshold(input: String, settings: &State<std::sync::Arc<std::sync::Mutex<Settings>>>) {
    let json_data: serde_json::Value =
        serde_json::from_str(input.as_str()).expect("Cannot parse JSON request body");
    let new_threshold = json_data
        .get("profit_threshold")
        .unwrap()
        .as_u64()
        .expect("Cannot parse unsigned int");
    settings.lock().unwrap().set_threshold(new_threshold)
}

#[post("/set_allowed_tokens", data = "<input>")]
pub fn set_allowed_tokens(
    input: String,
    settings: &State<std::sync::Arc<std::sync::Mutex<Settings>>>,
) {
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
        .set_allowed_tokens(new_allowed_token_accounts)
}

#[get("/profit")]
pub async fn profit(
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
pub async fn set_mapped_tokens(
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
pub async fn get_mapped_tokens(
    settings: &State<std::sync::Arc<std::sync::Mutex<Settings>>>,
) -> String {
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
pub async fn insert_mapped_tokens(
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
pub async fn remove_mapped_tokens(
    input: String,
    settings: &State<std::sync::Arc<std::sync::Mutex<Settings>>>,
) {
    settings.lock().unwrap().clone().remove_mapped_tokens(
        serde_json::from_str(&input).expect("Failed to parse JSON request body"),
    )
}
