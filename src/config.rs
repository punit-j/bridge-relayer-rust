use near_sdk::AccountId;
use serde_json::json;
use std::borrow::BorrowMut;
use std::fs;
use url::Url;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NearTokensCoinId {
    pub mapping: std::collections::HashMap<near_sdk::AccountId, String>,
}

impl NearTokensCoinId {
    pub fn get_coin_id(&self, near_token_account_id: near_sdk::AccountId) -> Option<String> {
        Some(self.mapping.get(&near_token_account_id)?.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EtherscanAPISettings {
    pub endpoint_url: url::Url,
    pub api_key: String,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LastBlockNumberWorkerSettings {
    pub server_addr: url::Url,
    pub contract_account_id: String,
    pub request_interval_secs: u64,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UnlockTokensWorkerSettings {
    pub server_addr: url::Url,
    pub contract_account_id: String,
    pub request_interval_secs: u64,
    pub blocks_for_tx_finalization: u64,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EthSettings {
    pub bridge_proxy_address: web3::types::Address,
    pub bridge_impl_address: web3::types::Address,
    pub private_key: String,
    pub rpc_url: Url,
    #[serde(default)]
    pub pending_transaction_poll_delay_sec: u32,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NearSettings {
    pub near_credentials_path: String,
    pub rpc_url: Url,
    pub contract_address: near_lake_framework::near_indexer_primitives::types::AccountId,
    pub allowed_tokens: Vec<AccountId>,
    pub near_lake_init_block: u64,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RedisSettings {
    pub url: Url,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub eth: EthSettings,
    pub near: NearSettings,
    pub redis: RedisSettings,
    pub profit_thershold: u64,
    pub vault_addr: Url,
    #[serde(skip)]
    pub config_path: String,
    pub etherscan_api: EtherscanAPISettings,
    pub last_block_number_worker: LastBlockNumberWorkerSettings,
    pub unlock_tokens_worker: UnlockTokensWorkerSettings,
    pub near_tokens_coin_id: NearTokensCoinId,
}

impl Settings {
    pub fn init(file_path: String) -> Result<Settings, String> {
        let path = std::path::Path::new(&file_path);
        if !path.exists() {
            return Err("Given config path doesn't exist".to_string());
        }

        let file = fs::File::open(path).map_err(|e| e.to_string())?;
        let reader = std::io::BufReader::new(file);

        // Read the JSON contents of the file as an instance of `User`.
        let mut config: Settings = serde_json::from_reader(reader).map_err(|e| e.to_string())?;
        config.config_path = file_path;
        Ok(config)
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
        let json_final: String = serde_json::to_string_pretty(&json).unwrap();
        fs::write(self.config_path.as_str(), &json_final).expect("Unable to write file");
    }

    pub fn set_threshold(&mut self, value: u64) {
        self.profit_thershold = value;
        self.set_json_value(vec!["profit_thershold".to_string()], json!(value));
    }

    pub fn set_allowed_tokens(&mut self, tokens: Vec<AccountId>) {
        self.near.allowed_tokens = tokens.clone();
        self.set_json_value(
            vec!["near".to_string(), "allowed_tokens".to_string()],
            json!(tokens),
        );
    }

    pub fn set_mapped_tokens(
        &mut self,
        mapped_tokens: std::collections::HashMap<near_sdk::AccountId, String>,
    ) {
        self.near_tokens_coin_id.mapping = mapped_tokens.clone();
        self.set_json_value(
            vec!["near_tokens_coin_id".to_string(), "mapping".to_string()],
            json!(mapped_tokens),
        );
    }

    pub fn insert_mapped_tokens(
        &mut self,
        mapped_tokens: std::collections::HashMap<near_sdk::AccountId, String>,
    ) {
        self.near_tokens_coin_id.mapping.extend(mapped_tokens);
        self.set_json_value(
            vec!["near_tokens_coin_id".to_string(), "mapping".to_string()],
            json!(self.near_tokens_coin_id.mapping),
        );
    }

    pub fn remove_mapped_tokens(&mut self, token_addresses: Vec<near_sdk::AccountId>) {
        for entry in token_addresses {
            self.near_tokens_coin_id.mapping.remove(&entry);
        }
        self.set_json_value(
            vec!["near_tokens_coin_id".to_string(), "mapping".to_string()],
            json!(self.near_tokens_coin_id.mapping),
        );
    }
}
