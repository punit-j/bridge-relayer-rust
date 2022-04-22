use crate::transfer_event;
extern crate redis;
use crate::config::RedisSettings;
use redis::{Commands, RedisResult};
use rocket::yansi::Color::Default;

pub struct RedisWrapper {
    client: redis::Client,
    connection: redis::Connection,
    settings: RedisSettings,
}

impl RedisWrapper {
    pub fn connect(settings: RedisSettings) -> Self {
        let c = redis::Client::open(settings.url.clone().unwrap())
            .expect(format!("Enable to open {}", settings.url.clone().unwrap()).as_str());
        let con = c.get_connection().expect("Unable to get connection");
        RedisWrapper {
            client: c,
            connection: con,
            settings: settings,
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

    #[test]
    fn read_write() {
        let event = super::transfer_event::tests::test_struct_build();

        let settings = RedisSettings {
            url: Some("redis://127.0.0.1/".to_string()),
        };

        let mut redis = super::RedisWrapper::connect(settings);
        redis.set(1, &event);

        assert!(redis.get(2).is_none());
        let res = redis.get(1).unwrap();

        super::transfer_event::tests::test_struct_check(&event, &res);
    }
}
