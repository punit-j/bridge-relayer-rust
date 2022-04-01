#[macro_use] extern crate rocket;

use std::io::Write;
use rocket::State;
use std::sync::atomic::{AtomicUsize, Ordering};
use redis::Commands;
use borsh::{BorshSerialize, BorshDeserialize};
use rocket::serde::Serialize;
use web3::types::H160;

struct HitCount {
    count: AtomicUsize
}

#[get("/health")]
fn health() -> String {
    "OK".to_string()
}

extern crate redis;

#[derive(BorshSerialize, BorshDeserialize/*, PartialEq, Debug*/)]
struct Transfer {
    token: web3::types::Address,
    amount: u128
}

impl BorshSerialize for web3::types::Address {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }
}


struct SpectreBridgeTransferEvent {
    nonce: u128,    // unique id of transaction,
    valid_till: u64,// unix_timestamp when transaction is expired,
    transfer: Transfer, // token account on ethereum side and eth amount
    fee: Transfer, // AccountId of token in which fee is paid and amount of fee paid to LP-Relayer for transferring
    recipient: web3::types::Address // recipient on Ethereum side
}

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

    let mut tt: H160;


    /*let mut rr = rocket::build();
    rr = rr.mount("/v1", routes![health]);
    rr.launch().await;
    */

}