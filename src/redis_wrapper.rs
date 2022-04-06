
use crate::transfer_event;
extern crate redis;
use redis::{Commands, RedisResult};

pub struct RedisWrapper {
   // client: redis::Connection

}

impl RedisWrapper {
    pub fn set(nonce: u128, event: transfer_event::SpectreBridgeTransferEvent) -> RedisResult<()> {
        let client = redis::Client::open("redis://127.0.0.1/")?;
        let mut con = client.get_connection()?;

        let serialize = serde_json::to_string(&event).unwrap();
        con.set(nonce.to_string(), serialize)?;

        Ok(())
    }

    pub fn get(nonce: u128) -> Option<transfer_event::SpectreBridgeTransferEvent> {
        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        let mut con = client.get_connection().unwrap();

        let res: RedisResult<String> = con.get(nonce.to_string());
        if res.is_ok() {
            let res = serde_json::from_str(&res.unwrap());
            if res.is_ok() {
                return res.unwrap();
            }
        }

        Option::None
    }
}