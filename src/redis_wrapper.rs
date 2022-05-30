use crate::transfer_event;
use std::ops::DerefMut;

extern crate redis;
use crate::config::RedisSettings;
use redis::{Commands, RedisResult};
use rocket::yansi::Color::Default;
use std::sync::Mutex;

const REDIS_TRANSACTION_HASH: &str = "myhash";
const REDIS_PROFIT_HASH: &str = "myprofit";

pub struct RedisWrapper {
    client: redis::Client,
    connection: Mutex<redis::Connection>,
}

impl RedisWrapper {
    pub fn connect(settings: RedisSettings) -> Self {
        let c = redis::Client::open(settings.url.clone())
            .expect(format!("Enable to open {}", settings.url.clone()).as_str());
        let con = c.get_connection().expect("Unable to get connection");
        RedisWrapper {
            client: c,
            connection: Mutex::new(con),
        }
    }

    pub fn set(
        &mut self,
        nonce: u128,
        event: &transfer_event::SpectreBridgeTransferEvent,
    ) -> RedisResult<()> {
        let serialize = serde_json::to_string(&event).expect("Unable to set value to the redis");
        self.connection.lock().unwrap().hset(
            REDIS_TRANSACTION_HASH,
            nonce.to_string(),
            serialize,
        )?;

        Ok(())
    }

    pub fn get(&mut self, nonce: u128) -> Option<transfer_event::SpectreBridgeTransferEvent> {
        let res: String = self
            .connection
            .lock()
            .unwrap()
            .hget(REDIS_TRANSACTION_HASH, nonce.to_string())
            .ok()?;

        serde_json::from_str(&res).expect("Unable to parse JSON")
    }

    pub fn get_all(&self) -> Vec<String> {
        let result: Vec<String> = self
            .connection
            .lock()
            .unwrap()
            .hvals(REDIS_TRANSACTION_HASH)
            .unwrap();
        result
    }

    pub fn _increase_profit(&mut self, add_to: u64) -> RedisResult<()> {
        let profit: i32 = self
            .connection
            .lock()
            .unwrap()
            .deref_mut()
            .hget(REDIS_PROFIT_HASH, "profit".to_string())
            .ok()
            .unwrap_or(0); // In case we don't have initial value in DB

        self.connection.lock().unwrap().hset(
            REDIS_PROFIT_HASH,
            "profit".to_string(),
            add_to + profit as u64,
        )?;

        Ok(())
    }

    pub fn get_profit(&self) -> u64 {
        self.connection
            .lock()
            .unwrap()
            .deref_mut()
            .hget(REDIS_PROFIT_HASH, "profit".to_string())
            .ok()
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::config::RedisSettings;
    use crate::transfer_event;
    use std::str::FromStr;
    use url::Url;

    #[test]
    fn read_write() {
        let event = super::transfer_event::tests::test_struct_build();

        let settings = RedisSettings {
            url: Url::parse("redis://127.0.0.1/").unwrap(),
        };

        let mut redis = super::RedisWrapper::connect(settings);
        redis.set(1, &event).unwrap();

        assert!(redis.get(2).is_none());
        let res = redis.get(1).unwrap();

        super::transfer_event::tests::test_struct_check(&event, &res);
    }

    #[test]
    fn read_all_transactions() {
        let settings = RedisSettings {
            url: Url::parse("redis://127.0.0.1/").unwrap(),
        };

        let mut redis = super::RedisWrapper::connect(settings);
        redis.get_all();
    }
}
