use config::{Config, File};
use std::env;
use std::path::Path;
use url::Url;

#[derive(Clone)]
pub struct EthSettings {
    pub private_key: Option<String>,
    pub rpc_url: Option<Url>,
    pub contract_address: Option<String>,
}

#[derive(Clone)]
pub struct NearSettings {
    pub private_key: Option<String>,
    pub rpc_url: Option<Url>,
    pub contract_address: Option<String>,
}

pub struct RedisSettings {
    pub url: Option<Url>,
}

pub struct Settings {
    pub eth_settings: EthSettings,
    pub near_settings: NearSettings,
    pub redis_setting: RedisSettings,
    pub profit_thershold: u64,
}

impl Settings {
    pub fn init(file_path: &Path) -> Self {
        let config = Config::builder()
            .add_source(File::with_name(file_path.to_str().unwrap()))
            .build()
            .unwrap();

        let eth_config = config.get_table("eth").unwrap();
        let eth = EthSettings {
            private_key: Some(eth_config.get("private_key").unwrap().to_string()),
            rpc_url: Some(
                Url::parse(eth_config.get("rpc_url").unwrap().to_string().as_str()).unwrap(),
            ),
            contract_address: Some(eth_config.get("contract_address").unwrap().to_string()),
        };

        let near_config = config.get_table("near").unwrap();
        let near = NearSettings {
            private_key: Some(near_config.get("private_key").unwrap().to_string()),
            rpc_url: Some(
                Url::parse(near_config.get("rpc_url").unwrap().to_string().as_str()).unwrap(),
            ),
            contract_address: Some(near_config.get("contract_address").unwrap().to_string()),
        };

        let redis = RedisSettings {
            url: Some(
                Url::parse(
                    config
                        .get_table("redis")
                        .unwrap()
                        .get("url")
                        .unwrap()
                        .to_string()
                        .as_str(),
                )
                .unwrap(),
            ),
        };

        let profit_thershold: u64 = config.get("profit_thershold").unwrap();

        Self {
            eth_settings: eth,
            near_settings: near,
            redis_setting: redis,
            profit_thershold,
        }
    }
}
