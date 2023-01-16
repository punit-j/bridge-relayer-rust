use serde_json::json;
use std::borrow::BorrowMut;
use std::fs;
use url::Url;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NearTokensWhitelist {
    pub mapping: std::collections::HashMap<near_sdk::AccountId, NearTokenInfo>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NearTokenInfo {
    pub exchange_id: String,
    pub fixed_fee: near_sdk::json_types::U128,
    pub percent_fee: f64,
    pub decimals: u32,
    pub eth_address: web3::types::Address,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NearNetwork {
    Mainnet,
    Testnet,
}

impl NearTokensWhitelist {
    pub fn get_token_info(&self, near_token_account_id: near_sdk::AccountId) -> Option<NearTokenInfo> {
        Some(self.mapping.get(&near_token_account_id)?.clone())
    }

    pub fn get_coin_id(&self, near_token_account_id: near_sdk::AccountId) -> Option<String> {
        Some(self.mapping.get(&near_token_account_id)?.exchange_id.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EtherscanAPISettings {
    pub endpoint_url: url::Url,
    pub api_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LastBlockNumberWorkerSettings {
    pub server_addr: url::Url,
    pub contract_account_id: String,
    pub request_interval_secs: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UnlockTokensWorkerSettings {
    pub server_addr: url::Url,
    pub contract_account_id: String,
    pub request_interval_secs: u64,
    pub blocks_for_tx_finalization: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EthSettings {
    pub bridge_proxy_address: web3::types::Address,
    pub bridge_impl_address: web3::types::Address,
    pub private_key: String,
    pub rpc_url: Url,
    #[serde(default)]
    pub pending_transaction_poll_delay_sec: u32,
    pub rainbow_bridge_index_js_path: String,
    pub num_of_confirmations: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NearSettings {
    pub near_credentials_path: String,
    pub rpc_url: Url,
    pub contract_address: near_lake_framework::near_indexer_primitives::types::AccountId,
    pub near_lake_init_block: u64,
    pub near_network: NearNetwork,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RedisSettings {
    pub url: Url,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub eth: EthSettings,
    pub near: NearSettings,
    pub redis: RedisSettings,
    pub profit_thershold: Option<f64>,
    pub vault_addr: Url,
    #[serde(skip)]
    pub config_path: String,
    pub etherscan_api: EtherscanAPISettings,
    pub last_block_number_worker: LastBlockNumberWorkerSettings,
    pub unlock_tokens_worker: UnlockTokensWorkerSettings,
    pub near_tokens_whitelist: NearTokensWhitelist,
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

    pub fn set_threshold(&mut self, value: Option<f64>) {
        self.profit_thershold = value;
        self.set_json_value(vec!["profit_thershold".to_string()], json!(value));
    }

    pub fn set_mapped_tokens(
        &mut self,
        mapped_tokens: std::collections::HashMap<near_sdk::AccountId, NearTokenInfo>,
    ) {
        self.near_tokens_whitelist.mapping = mapped_tokens.clone();
        self.set_json_value(
            vec!["near_tokens_whitelist".to_string(), "mapping".to_string()],
            json!(mapped_tokens),
        );
    }

    pub fn insert_mapped_tokens(
        &mut self,
        mapped_tokens: std::collections::HashMap<near_sdk::AccountId, NearTokenInfo>,
    ) {
        self.near_tokens_whitelist.mapping.extend(mapped_tokens);
        self.set_json_value(
            vec!["near_tokens_whitelist".to_string(), "mapping".to_string()],
            json!(self.near_tokens_whitelist.mapping),
        );
    }

    pub fn remove_mapped_tokens(&mut self, token_addresses: Vec<near_sdk::AccountId>) {
        for entry in token_addresses {
            self.near_tokens_whitelist.mapping.remove(&entry);
        }
        self.set_json_value(
            vec!["near_tokens_whitelist".to_string(), "mapping".to_string()],
            json!(self.near_tokens_whitelist.mapping),
        );
    }
}

#[cfg(test)]
pub mod tests {
    use crate::config::{Settings, NearTokenInfo};
    use crate::test_utils::get_settings;
    use near_sdk::AccountId;
    use std::collections::HashMap;
    use std::env::temp_dir;
    use std::fs;
    use uuid::Uuid;

    fn copy_config() -> String {
        let config_path = "config.json.example";

        let mut dir = temp_dir();
        let file_name = format!("{}.json", Uuid::new_v4());
        dir.push(file_name);

        let tmp_config_path = dir.to_str().unwrap();
        fs::copy(config_path, tmp_config_path).unwrap();

        tmp_config_path.to_string()
    }

    #[tokio::test]
    async fn smoke_init_test() {
        let config_path = "config.json.example";
        let settings = Settings::init(config_path.to_string()).unwrap();

        assert_eq!(
            settings.eth.rpc_url,
            url::Url::parse("https://goerli.infura.io/v3/${SPECTRE_BRIDGE_INFURA_PROJECT_ID}")
                .unwrap()
        );
        assert_eq!(settings.config_path, config_path);
        assert_eq!(
            settings.near.near_credentials_path,
            "~/.near-credentials/testnet/spectrebridge.testnet.json"
        );
        let token_account: near_sdk::AccountId = "6b175474e89094c44da98b954eedeac495271d0f.factory.bridge.near".parse().unwrap();
        assert_eq!(settings.near_tokens_whitelist.mapping[&token_account].fixed_fee.0, 340282366920938463463374607431768211455u128);
    }

    #[tokio::test]
    async fn smoke_get_coin_id_test() {
        let settings = get_settings();

        assert_eq!(
            settings
                .near_tokens_whitelist
                .get_coin_id(
                    AccountId::try_from("token.spectrebridge.testnet".to_string()).unwrap()
                )
                .unwrap(),
            "wrapped-near"
        );
    }

    #[tokio::test]
    async fn smoke_set_threshold_test() {
        let tmp_config_path = copy_config();

        let mut settings = Settings::init(tmp_config_path.clone()).unwrap();
        assert_eq!(settings.profit_thershold, Some(0.0));
        assert_eq!(settings.config_path, tmp_config_path.clone());

        settings.set_threshold(Some(10.0));
        assert_eq!(settings.profit_thershold, Some(10.0));

        let settings_new = Settings::init(tmp_config_path).unwrap();
        assert_eq!(settings_new.profit_thershold, Some(10.0));
    }

    #[tokio::test]
    async fn smoke_set_mapped_tokens_test() {
        let tmp_config_path = copy_config();

        let mut settings = Settings::init(tmp_config_path.clone()).unwrap();
        assert_eq!(settings.near_tokens_whitelist.mapping.len(), 4);
        assert_eq!(
            settings.near_tokens_whitelist.mapping
                [&AccountId::try_from("token.spectrebridge.testnet".to_string()).unwrap()].exchange_id,
            "wrapped-near".to_string()
        );

        let new_token_account_id =
            AccountId::try_from("new_token.bridge.near".to_string()).unwrap();
        settings.set_mapped_tokens(HashMap::from([(
            new_token_account_id,
            NearTokenInfo {
                exchange_id: "new_token".to_owned(),
                fixed_fee: 0.into(),
                percent_fee: 0.0,
                decimals: 18,
                eth_address: web3::types::H160::zero(),
            },
        )]));

        assert_eq!(settings.near_tokens_whitelist.mapping.len(), 1);
        assert_eq!(
            settings.near_tokens_whitelist.mapping
                [&AccountId::try_from("new_token.bridge.near".to_string()).unwrap()].exchange_id
                .as_str(),
            "new_token"
        );

        let settings_new = Settings::init(tmp_config_path).unwrap();
        assert_eq!(settings_new.near_tokens_whitelist.mapping.len(), 1);
        assert_eq!(
            settings_new.near_tokens_whitelist.mapping
                [&AccountId::try_from("new_token.bridge.near".to_string()).unwrap()].exchange_id
                .as_str(),
            "new_token"
        );
    }

    #[tokio::test]
    async fn smoke_insert_remove_mapped_tokens_test() {
        let tmp_config_path = copy_config();

        let mut settings = Settings::init(tmp_config_path.clone()).unwrap();
        assert_eq!(settings.near_tokens_whitelist.mapping.len(), 4);
        assert_eq!(
            settings.near_tokens_whitelist.mapping
                [&AccountId::try_from("token.spectrebridge.testnet".to_string()).unwrap()].exchange_id,
            "wrapped-near".to_string()
        );

        let new_token_account_id =
            AccountId::try_from("new_token.bridge.near".to_string()).unwrap();
        settings.insert_mapped_tokens(HashMap::from([(
            new_token_account_id.clone(),
            NearTokenInfo {
                exchange_id: "new_token".to_owned(),
                fixed_fee: 0.into(),
                percent_fee: 0.0,
                decimals: 18,
                eth_address: web3::types::H160::zero(),
            },
        )]));

        assert_eq!(settings.near_tokens_whitelist.mapping.len(), 5);
        assert_eq!(
            settings.near_tokens_whitelist.mapping
                [&AccountId::try_from("new_token.bridge.near".to_string()).unwrap()].exchange_id
                .as_str(),
            "new_token"
        );

        let settings_new = Settings::init(tmp_config_path.clone()).unwrap();
        assert_eq!(settings_new.near_tokens_whitelist.mapping.len(), 5);
        assert_eq!(
            settings_new.near_tokens_whitelist.mapping
                [&AccountId::try_from("new_token.bridge.near".to_string()).unwrap()].exchange_id
                .as_str(),
            "new_token"
        );

        settings.remove_mapped_tokens(vec![new_token_account_id.clone()]);
        assert_eq!(settings.near_tokens_whitelist.mapping.len(), 4);
        assert_eq!(
            settings
                .near_tokens_whitelist
                .get_coin_id(new_token_account_id.clone()),
            None
        );

        let settings_new_new = Settings::init(tmp_config_path.clone()).unwrap();
        assert_eq!(settings_new_new.near_tokens_whitelist.mapping.len(), 4);
        assert_eq!(
            settings_new_new
                .near_tokens_whitelist
                .get_coin_id(new_token_account_id),
            None
        );
    }
}
