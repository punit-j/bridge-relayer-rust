use config::{Config, File};
use near_sdk::AccountId;
use std::fs;
use std::path::Path;
use url::Url;

#[derive(Clone)]
pub struct EthSettings {
    pub private_key: String,
    pub rpc_url: Url,
    pub contract_address: String,
}

#[derive(Clone)]
pub struct NearSettings {
    pub private_key: String,
    pub rpc_url: Url,
    pub contract_address: AccountId,
}

#[derive(Clone)]
pub struct RedisSettings {
    pub url: Url,
}

pub struct Settings {
    pub eth_settings: EthSettings,
    pub near_settings: NearSettings,
    pub redis_setting: RedisSettings,
    pub profit_thershold: u64,

    pub config_path: String,
}

impl Settings {
    pub fn init(file_path: String) -> Self {
        let config_file_path = Path::new(&file_path);
        if !config_file_path.exists() {
            panic!("Given config path doesn't exist");
        }

        let config = Config::builder()
            .add_source(File::with_name(file_path.clone().as_str()))
            .build()
            .unwrap();

        let eth_config = config.get_table("eth").unwrap();
        let eth = EthSettings {
            private_key: eth_config.get("private_key").unwrap().to_string(),
            rpc_url: Url::parse(eth_config.get("rpc_url").unwrap().to_string().as_str()).unwrap(),

            contract_address: eth_config.get("contract_address").unwrap().to_string(),
        };

        let near_config = config.get_table("near").unwrap();
        let near = NearSettings {
            private_key: near_config.get("private_key").unwrap().to_string(),
            rpc_url: Url::parse(near_config.get("rpc_url").unwrap().to_string().as_str()).unwrap(),

            contract_address: AccountId::new_unchecked(
                near_config.get("contract_address").unwrap().to_string(),
            ),
        };

        let redis = RedisSettings {
            url: Url::parse(
                config
                    .get_table("redis")
                    .unwrap()
                    .get("url")
                    .unwrap()
                    .to_string()
                    .as_str(),
            )
            .unwrap(),
        };

        let profit_thershold: u64 = config.get("profit_thershold").unwrap();

        Self {
            eth_settings: eth,
            near_settings: near,
            redis_setting: redis,
            profit_thershold,
            config_path: file_path.clone(),
        }
    }

    pub fn set_single_value(&self, object: &str, value: u64) {
        let config_data =
            fs::read_to_string(self.config_path.as_str()).expect("Unable to read file");
        let mut json: serde_json::Value = serde_json::from_str(&config_data).unwrap();
        *json.get_mut(object).unwrap() = serde_json::json!(value);

        let json_final: String = serde_json::to_string(&json).unwrap();
        fs::write(self.config_path.as_str(), &json_final).expect("Unable to write file");
    }
}
