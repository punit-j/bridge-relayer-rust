mod async_redis_wrapper;
mod config;
mod errors;
mod ethereum;
mod event_processor;
mod last_block;
mod near;
mod pending_transactions_worker;
mod private_key;
mod profit_estimation;
mod transfer;
mod unlock_tokens;
mod utils;

use crate::config::Settings;
use clap::Parser;
use std::str::FromStr;
use uint::rustc_hex::ToHex;

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
}

#[allow(unused_must_use)]
#[tokio::main]
async fn main() {
    let args = Args::parse();

    let settings = match Settings::init(args.config) {
        Ok(settings) => std::sync::Arc::new(std::sync::Mutex::new(settings)),
        Err(msg) => panic!("{}", msg),
    };

    let async_redis = std::sync::Arc::new(std::sync::Mutex::new(
        async_redis_wrapper::AsyncRedisWrapper::connect(settings.clone()).await,
    ));

    let storage = std::sync::Arc::new(std::sync::Mutex::new(last_block::Storage::new()));

    // If args.eth_secret is valid then get key from it else from settings
    let eth_keypair = std::sync::Arc::new({
        if let Some(path) = args.eth_secret {
            secp256k1::SecretKey::from_str(path.as_str())
        } else {
            secp256k1::SecretKey::from_str(&settings.lock().unwrap().eth.private_key)
        }
        .expect("Unable to get an Eth key")
    });

    let eth_contract_address =
        std::sync::Arc::new(settings.lock().unwrap().clone().eth.bridge_proxy_address);

    let eth_contract_abi_settings = settings.lock().unwrap().clone();
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
            settings.lock().unwrap().near.near_credentials_path.as_str(),
        )
    }
    .unwrap();

    let near_contract_address = settings.lock().unwrap().near.contract_address.clone();

    let near_worker = near::run_worker(near_contract_address, async_redis.clone(), {
        let mut r = async_redis.lock().unwrap().clone();
        if let Some(b) = r.option_get::<u64>(near::OPTION_START_BLOCK).await.unwrap() {
            b
        } else {
            settings.lock().unwrap().near.near_lake_init_block
        }
    });

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
    );

    let pending_transactions_worker = utils::build_pending_transactions_worker(
        settings.clone(),
        eth_keypair.clone(),
        async_redis.lock().unwrap().clone(),
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
        async_redis.clone(),
    );

    let rocket_conf = rocket::Config::release_default();
    println!(
        "Starting rocket {:#?}:{}",
        &rocket_conf.address, &rocket_conf.port
    );
    let rocket = utils::build_rocket(rocket_conf, settings, storage, async_redis);

    tokio::join!(
        near_worker,
        subscriber,
        pending_transactions_worker,
        last_block_number_worker,
        unlock_tokens_worker,
        rocket.launch()
    );
}

#[cfg(test)]
pub mod tests {
    use crate::last_block;

    const APP_USER_AGENT: &str = "spectre-bridge-service/0.1.0";
    const NEAR_RPC_ENDPOINT_URL: &str = "https://rpc.testnet.near.org";
    const ETH_RPC_ENDPOINT_URL: &str =
        "https://goerli.infura.io/v3/ba5fd6c86e5c4e8c9b36f3f5b4013f7a";
    const ETHERSCAN_RPC_ENDPOINT_URL: &str = "https://api-goerli.etherscan.io";

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
