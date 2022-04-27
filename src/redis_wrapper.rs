use crate::transfer_event;
extern crate redis;
use crate::config::RedisSettings;
use redis::{Commands, RedisResult};
use rocket::yansi::Color::Default;
use url::Url;

pub struct RedisWrapper {
    client: redis::Client,
    connection: redis::Connection,
}

impl RedisWrapper {
    pub fn connect(settings: &RedisSettings) -> Self {
        let c = redis::Client::open(settings.url.clone())
            .expect(format!("Enable to open {}", settings.url.clone()).as_str());
        let con = c.get_connection().expect("Unable to get connection");
        RedisWrapper {
            client: c,
            connection: con,
        }
    }

    pub fn set(
        &mut self,
        nonce: u128,
        event: &transfer_event::SpectreBridgeTransferEvent,
    ) -> RedisResult<()> {
        let serialize = serde_json::to_string(&event).expect("Unable to set value to the redis");
        self.connection.set(nonce.to_string(), serialize)?;

        Ok(())
    }
    pub fn get(&mut self, nonce: u128) -> Option<transfer_event::SpectreBridgeTransferEvent> {
        let res: String = self.connection.get(nonce.to_string()).ok()?;

        serde_json::from_str(&res).expect("Unable to parce JSON")
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

        let mut redis = super::RedisWrapper::connect(&settings);
        redis.set(1, &event);

        assert!(redis.get(2).is_none());
        let res = redis.get(1).unwrap();

        super::transfer_event::tests::test_struct_check(&event, &res);
    }
}
