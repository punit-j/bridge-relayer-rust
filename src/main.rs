mod transfer_event;
mod redis_wrapper;

#[macro_use] extern crate rocket;

use std::io::Write;
use std::str::FromStr;
use rocket::State;
use std::sync::atomic::{AtomicUsize, Ordering};
use redis::Commands;
use borsh::{BorshSerialize, BorshDeserialize};
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
    let mut rr = rocket::build();
    rr = rr.mount("/v1", routes![health]);
    rr.launch().await;
}