mod profit_estimation;
mod redis_wrapper;
mod transfer_event;

#[macro_use]
extern crate rocket;

use crate::redis_wrapper::RedisWrapper;
use borsh::{BorshDeserialize, BorshSerialize};
use redis::Commands;
use rocket::State;
use std::io::Write;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};

struct HitCount {
    count: AtomicUsize,
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
