#[macro_use] extern crate rocket;

use std::io::Write;
use std::str::FromStr;
use rocket::State;
use std::sync::atomic::{AtomicUsize, Ordering};
use redis::Commands;
use borsh::{BorshSerialize, BorshDeserialize};
use rocket::serde::Serialize;

struct HitCount {
    count: AtomicUsize
}

#[get("/health")]
fn health() -> String {
    "OK".to_string()
}

extern crate redis;

pub struct SerializableAddress(pub web3::types::Address);
impl BorshSerialize for SerializableAddress {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(self.0.as_bytes())
    }
}
impl BorshDeserialize for SerializableAddress {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        if buf.len() < 20 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "wrong length",
            ));
        }
        let r = SerializableAddress::try_from_slice(buf)?;
        Ok(r)
    }
}

#[derive(BorshSerialize, BorshDeserialize/*, PartialEq, Debug*/)]
struct Transfer {
    token: SerializableAddress,//web3::types::Address,
    amount: u128
}


#[derive(BorshSerialize, BorshDeserialize)]
struct SpectreBridgeTransferEvent {
    nonce: u128,    // unique id of transaction,
    valid_till: u64,// unix_timestamp when transaction is expired,
    transfer: Transfer, // token account on ethereum side and eth amount
    fee: Transfer, // AccountId of token in which fee is paid and amount of fee paid to LP-Relayer for transferring
    recipient: SerializableAddress // recipient on Ethereum side
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

    let mut tt: SerializableAddress;// = SerializableAddress::from_str("123456789").unwrap() ;
    tt.0.

    /*let mut rr = rocket::build();
    rr = rr.mount("/v1", routes![health]);
    rr.launch().await;
    */

}