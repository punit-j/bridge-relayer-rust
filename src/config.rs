use dotenv::dotenv;
use std::{env, fs};
use url::Url;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "u32")]
pub struct Decimals(u32);

impl Decimals {
    const MAX_DECIMALS: u32 = 24;
}

impl TryFrom<u32> for Decimals {
    type Error = String;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value <= Decimals::MAX_DECIMALS {
            Ok(Self(value))
        } else {
            Err(format!(
                "The decimals value is too big. Max value = {}, found = {}",
                Decimals::MAX_DECIMALS,
                value
            ))
        }
    }
}

impl From<Decimals> for u32 {
    fn from(value: Decimals) -> Self {
        value.0
    }
}

impl From<Decimals> for usize {
    fn from(value: Decimals) -> Self {
        value.0.try_into().unwrap()
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NearTokensWhitelist {
    pub mapping: std::collections::HashMap<near_sdk::AccountId, NearTokenInfo>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NearTokenInfo {
    pub exchange_id: String,
    pub fixed_fee: near_sdk::json_types::U128,
    pub percent_fee: f64,
    pub decimals: Decimals,
    pub eth_address: web3::types::Address,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NearNetwork {
    Mainnet,
    Testnet,
}

impl NearTokensWhitelist {
    pub fn get_token_info(
        &self,
        near_token_account_id: near_sdk::AccountId,
    ) -> Option<NearTokenInfo> {
        Some(self.mapping.get(&near_token_account_id)?.clone())
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
    pub private_key: Option<String>,
    pub rpc_url: Url,
    #[serde(default)]
    pub pending_transaction_poll_delay_sec: u32,
    pub rainbow_bridge_index_js_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NearSettings {
    pub near_credentials_path: Option<String>,
    pub rpc_url: Url,
    pub contract_address: near_lake_framework::near_indexer_primitives::types::AccountId,
    pub near_lake_init_block: u64,
    pub near_network: NearNetwork,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RedisSettings {
    pub url: Url,
}

pub type SafeSettings = std::sync::Arc<tokio::sync::Mutex<Settings>>;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub eth: EthSettings,
    pub near: NearSettings,
    pub redis: RedisSettings,
    pub profit_thershold: Option<f64>,
    pub max_priority_fee_per_gas: Option<web3::types::U256>,
    pub min_time_before_unlock_in_sec: Option<u64>,
    pub vault_addr: Url,
    #[serde(skip)]
    pub config_path: String,
    pub etherscan_api: EtherscanAPISettings,
    pub last_block_number_worker: LastBlockNumberWorkerSettings,
    pub unlock_tokens_worker: UnlockTokensWorkerSettings,
    pub near_tokens_whitelist: NearTokensWhitelist,
    #[serde(default = "default_rpc_timeout_secs")]
    pub rpc_timeout_secs: u64,
    pub prometheus_metrics_port: Option<u16>,
}

pub fn default_rpc_timeout_secs() -> u64 {
    30
}

impl Settings {
    pub fn init(file_path: String) -> Result<Settings, String> {
        let path = std::path::Path::new(&file_path);
        if !path.exists() {
            return Err("Given config path doesn't exist".to_string());
        }

        let file = fs::File::open(path).map_err(|e| e.to_string())?;
        let reader = std::io::BufReader::new(file);

        dotenv().ok();

        // Read the JSON contents of the file as an instance of `User`.
        let mut config: Settings = serde_json::from_reader(reader).map_err(|e| e.to_string())?;
        config.config_path = file_path;
        if let Some(eth_private_key) = config.eth.private_key {
            config.eth.private_key = Some(eth_private_key.replace(
                "${FAST_BRIDGE_ETH_PRIVATE_KEY}",
                &env::var("FAST_BRIDGE_ETH_PRIVATE_KEY").unwrap_or("".to_string()),
            ));
        }
        config.eth.rpc_url = url::Url::parse(&config.eth.rpc_url.as_str().replace(
            "FAST_BRIDGE_INFURA_PROJECT_ID",
            &env::var("FAST_BRIDGE_INFURA_PROJECT_ID").unwrap_or("".to_string()),
        ))
        .unwrap();
        config.etherscan_api.api_key = config.etherscan_api.api_key.replace(
            "${FAST_BRIDGE_ETHERSCAN_API_KEY}",
            &env::var("FAST_BRIDGE_ETHERSCAN_API_KEY").unwrap_or("".to_string()),
        );

        Ok(config)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::config::Settings;

    #[tokio::test]
    async fn smoke_init_test() {
        let config_path = "config.json.example";
        let settings = Settings::init(config_path.to_string()).unwrap();

        assert_ne!(
            settings.eth.rpc_url,
            url::Url::parse("https://goerli.infura.io/v3/FAST_BRIDGE_INFURA_PROJECT_ID").unwrap()
        );
        assert_eq!(settings.config_path, config_path);
        assert_eq!(
            settings.near.near_credentials_path.unwrap(),
            "~/.near-credentials/testnet/fastbridge.testnet.json"
        );
        let token_account: near_sdk::AccountId =
            "6b175474e89094c44da98b954eedeac495271d0f.factory.bridge.near"
                .parse()
                .unwrap();
        assert_eq!(
            settings.near_tokens_whitelist.mapping[&token_account]
                .fixed_fee
                .0,
            340282366920938463463374607431768211455u128
        );
    }
}
