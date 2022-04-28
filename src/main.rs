mod config;
mod redis_wrapper;
mod transfer_event;

#[macro_use]
extern crate rocket;

use crate::config::Settings;
use crate::redis_wrapper::RedisWrapper;
use rocket::State;
use serde_json::json;
use std::env;

#[get("/health")]
fn health() -> String {
    "OK".to_string()
}

#[get("/transactions")]
fn transactions(settings: &State<Settings>) -> String {
    let redis = RedisWrapper::connect(settings.redis_setting.clone());
    let res = redis.get_all();

    json!(res).to_string()
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

    settings.set_single_value("profit_thershold", new_threshold);
}

extern crate redis;

#[rocket::main]
async fn main() {
    // Reading arguments that was given to binary
    let args: Vec<String> = env::args().collect();
    let config_file_path = args.get(1).unwrap().to_string();

    let settings = Settings::init(config_file_path);

    let _res = rocket::build()
        .mount("/v1", routes![health, transactions, set_threshold])
        .manage(settings)
        .launch()
        .await;
}
