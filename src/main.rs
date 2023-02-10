mod async_redis_wrapper;
mod config;
mod errors;
mod ethereum;
mod event_processor;
mod last_block;
mod logs;
mod near;
mod pending_transactions_worker;
mod private_key;
mod profit_estimation;
mod transfer;
mod unlock_tokens;
mod utils;

#[cfg(test)]
mod test_utils;

use crate::config::Settings;
use crate::logs::init_logger;
use clap::Parser;
use std::str::FromStr;

extern crate redis;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// config file
    #[clap(short, long)]
    config: String,

    /// eth secret key
    #[clap(long)]
    eth_secret: Option<String>,

    /// path to json file
    #[clap(long)]
    near_credentials: Option<String>,

    /// to override the value from redis
    #[clap(long)]
    near_lake_init_block: Option<u64>,
}

async fn check_system_time(near_rpc_url: url::Url) {
    const MAX_TIMESTAMP_DIFF_SEC: u64 = 60;

    let near_timestamp_ns = near_client::methods::get_final_block_timestamp(near_rpc_url)
        .await
        .expect("Error on getting NEAR block timestamp");
    let near_timestamp_sec = std::time::Duration::from_nanos(near_timestamp_ns).as_secs();

    let sys_timestamp_sec = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if sys_timestamp_sec + MAX_TIMESTAMP_DIFF_SEC < near_timestamp_sec
        || near_timestamp_sec + MAX_TIMESTAMP_DIFF_SEC < sys_timestamp_sec
    {
        panic!(
            "Incorrect UNIX timestamp. NEAR timestamp = {}, sys timestamp = {}",
            near_timestamp_sec, sys_timestamp_sec
        );
    }
}

#[allow(unused_must_use)]
#[tokio::main]
async fn main() {
    let args = Args::parse();

    init_logger();

    let settings = match Settings::init(args.config) {
        Ok(settings) => std::sync::Arc::new(tokio::sync::Mutex::new(settings)),
        Err(msg) => panic!("{}", msg),
    };

    check_system_time(settings.lock().await.near.rpc_url.clone()).await;

    let mut async_redis =
        async_redis_wrapper::AsyncRedisWrapper::connect(&settings.lock().await.redis).await;

    let storage = std::sync::Arc::new(tokio::sync::Mutex::new(last_block::Storage::new()));

    // If args.eth_secret is valid then get key from it else from settings
    let eth_keypair = std::sync::Arc::new({
        if let Some(path) = args.eth_secret {
            secp256k1::SecretKey::from_str(path.as_str())
        } else {
            secp256k1::SecretKey::from_str(&settings.lock().await.eth.private_key)
        }
        .expect("Unable to get an Eth key")
    });

    let eth_contract_address = std::sync::Arc::new(settings.lock().await.eth.bridge_proxy_address);

    let eth_contract_abi_settings = settings.lock().await.clone();
    let eth_contract_abi = std::sync::Arc::new(
        eth_client::methods::get_contract_abi(
            eth_contract_abi_settings
                .etherscan_api
                .endpoint_url
                .as_ref(),
            eth_contract_abi_settings.eth.bridge_impl_address,
            &eth_contract_abi_settings.etherscan_api.api_key,
        )
        .await
        .expect("Failed to get contract abi"),
    );

    let near_account = if let Some(path) = args.near_credentials {
        near_client::read_private_key::read_private_key_from_file(path.as_str())
    } else {
        near_client::read_private_key::read_private_key_from_file(
            settings.lock().await.near.near_credentials_path.as_str(),
        )
    }
    .unwrap();

    let near_contract_address = settings.lock().await.near.contract_address.clone();

    let near_worker = near::run_worker(
        near_contract_address,
        async_redis.clone(),
        {
            if let Some(start_block) = args.near_lake_init_block {
                start_block
            } else if let Some(start_block) = async_redis
                .option_get::<u64>(near::OPTION_START_BLOCK)
                .await
                .unwrap()
            {
                start_block
            } else {
                settings.lock().await.near.near_lake_init_block
            }
        },
        settings.lock().await.near.near_network.clone(),
    );

    let stream = async_redis_wrapper::subscribe::<String>(
        async_redis_wrapper::EVENTS.to_string(),
        async_redis.clone(),
    )
    .unwrap();

    let subscriber = utils::build_near_events_subscriber(
        settings.clone(),
        eth_keypair.clone(),
        async_redis.clone(),
        eth_contract_abi.clone(),
        eth_contract_address.clone(),
        stream,
        near_account.account_id.to_string(),
    );

    let pending_transactions_worker = utils::build_pending_transactions_worker(
        settings.lock().await.clone(),
        eth_keypair.clone(),
        async_redis.clone(),
        eth_contract_abi.clone(),
        eth_contract_address.clone(),
    );

    let last_block_number_worker =
        last_block::last_block_number_worker(settings.clone(), storage.clone());

    let unlock_tokens_worker = unlock_tokens::unlock_tokens_worker(
        near_account.clone(),
        300_000_000_000_000,
        settings.clone(),
        storage.clone(),
        async_redis,
    );

    let tasks = vec![
        tokio::spawn(last_block_number_worker),
        tokio::spawn(subscriber),
        tokio::spawn(pending_transactions_worker),
        tokio::spawn(unlock_tokens_worker),
        tokio::spawn(near_worker),
    ];

    for task in tasks {
        task.await;
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{check_system_time, last_block};
    use std::str::FromStr;

    const APP_USER_AGENT: &str = "fast-bridge-service/0.1.0";
    const NEAR_RPC_ENDPOINT_URL: &str = "https://rpc.testnet.near.org";
    const ETH_RPC_ENDPOINT_URL: &str =
        "https://goerli.infura.io/v3/ba5fd6c86e5c4e8c9b36f3f5b4013f7a";
    const ETHERSCAN_RPC_ENDPOINT_URL: &str = "https://api-goerli.etherscan.io";

    #[tokio::test]
    async fn check_sys_time_test() {
        check_system_time(url::Url::from_str(NEAR_RPC_ENDPOINT_URL).unwrap()).await;
    }

    #[tokio::test]
    async fn near_rpc_status() {
        let client = near_jsonrpc_client::JsonRpcClient::connect(NEAR_RPC_ENDPOINT_URL);
        let status = client
            .call(near_jsonrpc_client::methods::status::RpcStatusRequest)
            .await;
        assert!(
            matches!(
                status,
                Ok(near_jsonrpc_client::methods::status::RpcStatusResponse { .. })
            ),
            "expected an Ok(RpcStatusResponse), found [{:?}]",
            status
        );
    }

    #[tokio::test]
    pub async fn eth_rpc_status() {
        let transport = web3::transports::Http::new(ETH_RPC_ENDPOINT_URL);
        assert!(transport.is_ok(), "{:?}", transport.unwrap_err());
    }

    #[tokio::test]
    pub async fn etherscan_rpc_status() {
        let client = reqwest::Client::builder()
            .user_agent(APP_USER_AGENT)
            .build();
        assert!(client.is_ok(), "{:?}", client.unwrap_err());
        let res = client.unwrap().get(ETHERSCAN_RPC_ENDPOINT_URL).send().await;
        assert!(res.is_ok(), "{:?}", res.unwrap_err());
        assert_eq!(reqwest::StatusCode::OK, res.unwrap().status());
    }

    #[tokio::test]
    pub async fn last_block_number() {
        let result = last_block::last_block_number(
            NEAR_RPC_ENDPOINT_URL.try_into().unwrap(),
            "client-eth2.goerli.testnet".to_string(),
        )
        .await;

        assert!(result.unwrap().unwrap() >= 8129711);
    }
}
