mod transfer_event;
mod redis_wrapper;
mod config;

#[macro_use] extern crate rocket;

use std::io::Write;
use std::str::FromStr;
use rocket::State;
use std::sync::atomic::{AtomicUsize, Ordering};
use redis::Commands;
use borsh::{BorshSerialize, BorshDeserialize};
use std::env;
use std::path::Path;
use serde_json::json;
use crate::config::Settings;
use crate::redis_wrapper::RedisWrapper;

struct HitCount {
    count: AtomicUsize
}

#[get("/health")]
fn health() -> String {
    "OK".to_string()
}

#[get("/transactions")]
fn transactions(settings: &State<Settings>) -> String {
    let mut redis = RedisWrapper::connect(settings.redis_setting.clone());
    let res = redis.get_all();

    json!(res).to_string()
}

extern crate redis;

#[rocket::main]
async fn main() {
    // Reading arguments that was given to binary
    let args: Vec<String> = env::args().collect();
    let config_file_path = Path::new(args.get(1).unwrap());

    if !config_file_path.exists()
    {
        panic!("Given config path doesn't exist");
    }

    let settings = Settings::init(config_file_path);
    let mut redis = RedisWrapper::connect(settings.redis_setting.clone());

    let mut rr = rocket::build()
    .mount("/v1", routes![health, transactions])
    .manage(settings)
    .manage(redis)
    .launch().await;
}