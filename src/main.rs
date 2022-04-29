mod transfer_event;
mod redis_wrapper;
mod near;

#[macro_use] extern crate rocket;

use std::io::Write;
use std::str::FromStr;
use rocket::State;
use std::sync::atomic::{AtomicUsize, Ordering};
use redis::Commands;
use borsh::{BorshDeserialize, BorshSerialize};
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
    near::run_watcher().await;


    let mut rr = rocket::build();
    rr = rr.mount("/v1", routes![health]);
    rr.launch().await;
}