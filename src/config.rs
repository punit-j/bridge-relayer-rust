use config::{Config, File};
use std::env;

pub struct EthSettings {
    pub private_key: Option<String>,
    pub rpc_url: Option<String>,
    pub contract_address: Option<String>,
}

pub struct NearSettings {
    pub private_key: Option<String>,
    pub rpc_url: Option<String>,
    pub contract_address: Option<String>,
}

pub struct RedisSettings {
    pub url: Option<String>,
}

pub struct Settings {
    eth_settings: EthSettings,
    near_settings: NearSettings,
    redis_setting: RedisSettings,
    profit_threshold: u64,
}

impl Settings {
    pub fn init(file_path: &String) -> Self {
        let config = Config::builder()
            .add_source(File::with_name(file_path.as_str()))
            .build()
            .unwrap();

        // Will be filled in further sub task
        let eth = EthSettings {
            private_key: None,
            rpc_url: None,
            contract_address: None,
        };

        // Will be filled in further sub task
        let near = NearSettings {
            private_key: None,
            rpc_url: None,
            contract_address: None,
        };

        let redis = RedisSettings {
            url: Some(
                config
                    .get_table("redis")
                    .unwrap()
                    .get("url")
                    .unwrap()
                    .to_string(),
            ),
        };

        Self {
            eth_settings: eth,
            near_settings: near,
            redis_setting: redis,
            profit_threshold: 0,
        }
    }
}
