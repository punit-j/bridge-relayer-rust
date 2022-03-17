#[macro_use] extern crate rocket;
use rocket::State;
use std::sync::atomic::{AtomicUsize, Ordering};

struct HitCount {
    count: AtomicUsize
}

#[get("/health")]
fn health() -> String {
    "OK".to_string()
}

#[rocket::main]
async fn main() {
    let mut rr = rocket::build();
    rr = rr.mount("/v1", routes![health]);
    rr.launch().await;
}