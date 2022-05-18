use config::{Config, File};
use near_sdk::AccountId;
use redis::Value;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::{Borrow, BorrowMut};
use std::fs;
use std::ops::DerefMut;
use std::path::Path;
use std::sync::Mutex;
use url::Url;

#[derive(Clone)]
pub struct EthSettings {
    pub private_key: String,
    pub rpc_url: Url,
    pub contract_address: String,
}

pub struct NearSettings {
    pub private_key: String,
    pub rpc_url: Url,
    pub contract_address: AccountId,
    pub allowed_tokens: Mutex<Vec<AccountId>>,
}

#[derive(Clone)]
pub struct RedisSettings {
    pub url: Url,
}

pub struct Settings {
    pub eth_settings: EthSettings,
    pub near_settings: NearSettings,
    pub redis_setting: RedisSettings,
    pub profit_thershold: Mutex<u64>,
    pub vault_addr: Url,
    pub config_path: String,
    pub worker_interval: u64,
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
        let allowed_tokens: Vec<config::Value> = near_config
            .get("allowed_tokens")
            .unwrap()
            .clone()
            .into_array()
            .unwrap();

        let mut token_accounts: Vec<AccountId> = Vec::new();
        for val in allowed_tokens.iter() {
            token_accounts.push(AccountId::new_unchecked(val.to_string()));
        }

        let near = NearSettings {
            private_key: near_config.get("private_key").unwrap().to_string(),
            rpc_url: Url::parse(near_config.get("rpc_url").unwrap().to_string().as_str()).unwrap(),

            contract_address: AccountId::new_unchecked(
                near_config.get("contract_address").unwrap().to_string(),
            ),
            allowed_tokens: Mutex::new(token_accounts),
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

        // Example: http://127.0.0.1:8200/v1/kv/
        // `v1` - version of secrets that are using
        // `kv` - name of secret engine that must be initialized before use
        let vault_addr: String = config.get("vault_addr").unwrap();

        let profit_thershold: u64 = config.get("profit_thershold").unwrap();
        Self {
            eth_settings: eth,
            near_settings: near,
            redis_setting: redis,
            profit_thershold: Mutex::new(profit_thershold),
            vault_addr: Url::parse(&vault_addr).unwrap(),
            config_path: file_path.clone(),
            worker_interval: 15,
        }
    }

    fn set_json_value(&self, fields: Vec<String>, value: serde_json::Value) {
        let config_data =
            fs::read_to_string(self.config_path.as_str()).expect("Unable to read file");
        let mut json: serde_json::Value = serde_json::from_str(&config_data).unwrap();

        let mut nested_value: &mut serde_json::Value = json.borrow_mut();
        for val in fields.clone() {
            if fields.last().unwrap().eq(&val) {
                *nested_value.get_mut(val).unwrap() = value.clone();
            } else {
                nested_value = nested_value.get_mut(val).unwrap().borrow_mut();
            }
        }

        let json_final: String = serde_json::to_string(&json).unwrap();
        fs::write(self.config_path.as_str(), &json_final).expect("Unable to write file");
    }

    pub fn set_threshold(&self, value: u64) {
        self.set_json_value(vec!["profit_thershold".to_string()], json!(value));
    }

    pub fn set_allowed_tokens(&self, tokens: Vec<AccountId>) {
        *self
            .near_settings
            .allowed_tokens
            .lock()
            .unwrap()
            .deref_mut() = tokens.clone();

        self.set_json_value(
            vec!["near".to_string(), "allowed_tokens".to_string()],
            json!(tokens),
        );
    }
}
