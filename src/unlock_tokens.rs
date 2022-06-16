async fn unlock_tokens(
    server_addr: url::Url,
    account: near_crypto::InMemorySigner,
    contract_account_id: String,
    proof: spectre_bridge_common::Proof,
    nonce: u128,
    gas: u64,
) -> near_primitives::views::FinalExecutionStatus {
    let response = near_client::methods::change(
        server_addr,
        account,
        contract_account_id,
        "lp_unlock".to_string(),
        near_sdk::serde_json::json!({
            "nonce": near_sdk::json_types::U128(nonce),
            "proof": proof,
        }),
        gas,
        0,
    )
        .await
        .expect("Failed to fetch response by calling lp_unlock contract method");
    response.status
}

pub async fn unlock_tokens_worker(
    account: near_crypto::InMemorySigner,
    gas: u64,
    settings: std::sync::Arc<std::sync::Mutex<crate::Settings>>,
    storage: std::sync::Arc<std::sync::Mutex<crate::last_block::Storage>>,
    redis: std::sync::Arc<std::sync::Mutex<crate::async_redis_wrapper::AsyncRedisWrapper>>,
) -> redis::RedisResult<()> {
    tokio::spawn(async move {
        let mut connection = redis.lock().unwrap().clone();
        loop {
            let unlock_tokens_worker_settings =
                settings.lock().unwrap().unlock_tokens_worker.clone();
            crate::utils::request_interval(unlock_tokens_worker_settings.request_interval_secs)
                .await
                .tick()
                .await;
            match connection
                .lindex()
                .await
                .expect("REDIS: Failed to pop first tx_hash in queue")
            {
                Some(tx_hash) => {
                    let last_block_number = storage.lock().unwrap().clone().last_block_number;
                    let tx_data = connection
                        .hget(tx_hash.clone())
                        .await
                        .expect("REDIS: Failed to get TransactionData by tx_hash from set");
                    match tx_data.block + unlock_tokens_worker_settings.some_blocks_number
                        <= last_block_number
                    {
                        true => {
                            let status = crate::unlock_tokens::unlock_tokens(
                                unlock_tokens_worker_settings.server_addr,
                                account.clone(),
                                unlock_tokens_worker_settings.contract_account_id,
                                tx_data.proof,
                                tx_data.nonce,
                                gas,
                            )
                                .await;
                            if let near_primitives::views::FinalExecutionStatus::SuccessValue(_) = status {
                                connection
                                .lpop()
                                .await
                                .expect("REDIS: Failed to delete element by tx_hash from queue");
                                connection
                                .hdel(tx_hash.clone())
                                .await
                                .expect("REDIS: Failed to delete element by tx_hash from set");
                            }
                        }
                        false => connection
                            .rpush(tx_hash.clone())
                            .await
                            .expect("REDIS: failed to enqueue tx_hash"),
                    }
                }
                None => (),
            }
        }
    });
    Ok(())
}

#[cfg(test)]
pub mod tests {

    #[tokio::test]
    pub async fn unlock_tokens() {
        let response = super::unlock_tokens(
            url::Url::parse("https://rpc.testnet.near.org").unwrap(),
            near_client::read_private_key::read_private_key_from_file(
                "/home/arseniyk/.near-credentials/testnet/arseniyrest.testnet.json",
            ).unwrap(),
            "transfer.spectrebridge.testnet".to_string(),
            spectre_bridge_common::Proof::default(),
            909090,
            300_000_000_000_000,
        )
            .await;
        assert_eq!(response.as_success().is_some(), true);
    }
}
