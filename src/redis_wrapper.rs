
use crate::transfer_event;
extern crate redis;
use redis::{Commands, RedisResult};
use rocket::yansi::Color::Default;

pub struct RedisWrapper {
   client: redis::Client,
   connection: redis::Connection
}

impl RedisWrapper {
   pub fn connect() -> Self {
       let c = redis::Client::open("redis://127.0.0.1/").unwrap();
       let mut con = c.get_connection().unwrap();
       let v = RedisWrapper { client: c, connection: con };
       v
    }

    pub fn set(&mut self, nonce: u128, event: &transfer_event::SpectreBridgeTransferEvent) -> RedisResult<()> {
        let serialize = serde_json::to_string(&event).unwrap();
        self.connection.set(nonce.to_string(), serialize)?;

        Ok(())
    }

    pub fn get(&mut self, nonce: u128) -> Option<transfer_event::SpectreBridgeTransferEvent> {
        let res: RedisResult<String> = self.connection.get(nonce.to_string());
        if res.is_ok() {
            let res = serde_json::from_str(&res.unwrap());
            if res.is_ok() {
                return res.unwrap();
            }
        }

        Option::None
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use crate::transfer_event;

    #[test]
    fn read_write() {
        let event = super::transfer_event::tests::test_struct_build();

        let mut redis = super::RedisWrapper::connect();
        redis.set(1, &event);

        assert!(redis.get(2).is_none());
        let res = redis.get(1).unwrap();

        super::transfer_event::tests::test_struct_check(&event, &res);
    }
}