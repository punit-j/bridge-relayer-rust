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
use crate::config::Settings;
use crate::redis_wrapper::RedisWrapper;

struct HitCount {
    count: AtomicUsize
}

#[get("/health")]
fn health() -> String {
    "OK".to_string()
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

    // TODO: Probably it's better to hide all this unpacking to submodules, but didn't find a proper way to include config structures
    let near_settings = settings.near_settings.clone();
    let near_client = near_client::methods::NearClient::init(
        near_settings.private_key,
        near_settings.rpc_url,
        near_settings.contract_address,
    );

    let eth_settings = settings.near_settings.clone();
    let eth_client = eth_client::methods::EthClient::init(
        eth_settings.private_key,
        eth_settings.rpc_url,
        eth_settings.contract_address.to_string(),
    );

    let mut rr = rocket::build();
    rr = rr.mount("/v1", routes![health]);
    rr.launch().await;
}