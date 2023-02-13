use crate::config::{Decimals, NearTokenInfo, Settings};
use dotenv::dotenv;
use redis::AsyncCommands;
use std::env;
use std::path::Path;
use std::time::Duration;

pub const NEAR_CONTRACT_ADDRESS: &str = "fast-bridge2.olga24912_3.testnet";

pub fn get_valid_till() -> u64 {
    (std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        + Duration::from_secs(3 * 60 * 60).as_nanos()) as u64
}

pub async fn remove_all(redis: crate::async_redis_wrapper::AsyncRedisWrapper, key: &str) {
    let mut redis_connection = redis.connection;
    let mut iter: redis::AsyncIter<(String, String)> = redis_connection.hscan(key).await.unwrap();

    let mut keys = vec![];
    while let Some(pair) = iter.next_item().await {
        keys.push(pair.0);
    }

    for hash in keys {
        let _res: () = redis_connection.hdel(key, hash).await.unwrap();
    }
}

pub fn get_rb_index_path_str() -> String {
    dotenv().ok();
    let path_to_rainbow_bridge_rep_str = env::var("PATH_TO_RAINBOW_BRIDGE_REP").unwrap();
    let path_to_rainbow_bridge_rep = Path::new(&path_to_rainbow_bridge_rep_str);
    let rb_index_path = path_to_rainbow_bridge_rep.join("cli/index.js");
    rb_index_path.to_str().unwrap().to_string()
}

pub fn get_settings() -> Settings {
    let config_path = "config.json.example";
    let mut settings = crate::config::Settings::init(config_path.to_string()).unwrap();
    settings.near_tokens_whitelist.mapping.insert(
        near_client::test_utils::NEAR_TOKEN_ADDRESS.parse().unwrap(),
        NearTokenInfo {
            exchange_id: "wrapped-near".to_string(),
            fixed_fee: 0.into(),
            percent_fee: 0.0,
            decimals: Decimals::try_from(6).unwrap(),
            eth_address: eth_client::test_utils::ETH_TOKEN_ADDRESS.parse().unwrap(),
        },
    );
    settings
}
