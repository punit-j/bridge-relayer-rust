mod redis_storage;

#[macro_use] extern crate rocket;

use std::io::Write;
use std::str::FromStr;
use rocket::State;
use std::sync::atomic::{AtomicUsize, Ordering};
use redis::Commands;
use borsh::{BorshSerialize, BorshDeserialize};

struct HitCount {
    count: AtomicUsize
}

#[get("/health")]
fn health() -> String {
    "OK".to_string()
}

extern crate redis;

fn do_something() -> redis::RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let mut con = client.get_connection()?;

    //con.set("kkk", "vvv")?;
    let sss : String = con.get("kkk")?;
    println!("qq {}", sss);


    Ok(())
}

#[rocket::main]
async fn main() {
    do_something();

    /*let mut rr = rocket::build();
    rr = rr.mount("/v1", routes![health]);
    rr.launch().await;
    */

}