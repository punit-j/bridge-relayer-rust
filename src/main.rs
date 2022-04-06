mod transfer_event;
mod redis_wrapper;

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
    
    let event = transfer_event::SpectreBridgeTransferEvent{
        valid_till: 2,
        transfer: transfer_event::Transfer { token: Default::default(), amount: 0 },
        fee: transfer_event::Transfer { token: Default::default(), amount: 0 },
        recipient: Default::default()
    };
    
    redis_wrapper::RedisWrapper::set(1, event);
    let res = redis_wrapper::RedisWrapper::get(1);

    println!("{:?}", res);

    /*let mut rr = rocket::build();
    rr = rr.mount("/v1", routes![health]);
    rr.launch().await;
    */

}