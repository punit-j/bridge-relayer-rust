use redis::AsyncCommands;
use std::env;
use crate::config::{NearTokenInfo, Settings};
use std::path::Path;

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
    let path_to_rainbow_bridge_rep_str = env::var("PATH_TO_RAINBOW_BRIDGE_REP").unwrap();
    let path_to_rainbow_bridge_rep = Path::new(&path_to_rainbow_bridge_rep_str);
    let rb_index_path = path_to_rainbow_bridge_rep.join("cli/index.js");
    rb_index_path.to_str().unwrap().to_string()
}

pub fn get_settings() -> Settings {
    let config_path = "config.json.example";
    let mut settings = crate::config::Settings::init(config_path.to_string()).unwrap();
    settings.eth.num_of_confirmations = 1;
    settings.near_tokens_whitelist.mapping.insert(near_client::test_utils::NEAR_TOKEN_ADDRESS.parse().unwrap(),
                                                  NearTokenInfo{
                                                      exchange_id: "wrapped-near".to_string(),
                                                      fixed_fee: 0.into(),
                                                      percent_fee: 0.0,
                                                      decimals: 6,
                                                      eth_address: eth_client::test_utils::ETH_TOKEN_ADDRESS.parse().unwrap()
                                                  }
    );
    settings
}