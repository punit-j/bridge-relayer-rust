pub async fn unlock_tokens(
    server_addr: String,
    signer_account_id: String,
    signer_secret_key: String,
    contract_address: String,
    proof: spectre_bridge_common::Proof,
    nonce: u128,
    gas: u64,
) -> near_primitives::views::FinalExecutionStatus {
    let response = near_client::methods::change(
        server_addr,
        signer_account_id,
        signer_secret_key,
        contract_address,
        "lp_unlock".to_string(),
        near_sdk::serde_json::json!({
            "proof": proof,
            "nonce": near_sdk::json_types::U128(nonce),
        }),
        gas,
        0,
    )
    .await
    .expect("Failed to fetch response by calling lp_unlock contract method");
    response.status
}

pub async fn unlock_tokens_worker(
    server_addr: String,
    signer_account_id: String,
    signer_secret_key: String,
    contract_address: String,
    gas: u64,
    request_interval_sec: u64,
    some_number: u64,
    storage: std::sync::Arc<std::sync::Mutex<crate::last_block::Storage>>,
    redis: std::sync::Arc<std::sync::Mutex<crate::async_redis_wrapper::AsyncRedisWrapper>>,
) -> redis::RedisResult<()> {
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(request_interval_sec));
        let mut connection = redis.lock().unwrap().clone();
        loop {
            match connection
                .lpop()
                .await
                .expect("REDIS: Failed to pop first tx_hash in queue")
            {
                Some(tx_hash) => {
                    let mut last_block_number: u64;
                    {
                        let mut storage = storage.lock().unwrap();
                        last_block_number = *storage.last_block_number.lock().unwrap();
                    }
                    let tx_data = connection
                        .hget(tx_hash.clone())
                        .await
                        .expect("REDIS: Failed to get TransactionData by tx_hash from set");
                    match tx_data.block + some_number <= last_block_number {
                        true => {
                            crate::unlock_tokens::unlock_tokens(
                                server_addr.clone(),
                                signer_account_id.clone(),
                                signer_secret_key.clone(),
                                contract_address.clone(),
                                tx_data.proof,
                                tx_data.nonce,
                                gas,
                            )
                            .await;
                            connection
                                .hdel(tx_hash.clone())
                                .await
                                .expect("REDIS: Failed to delete element by tx_hash from set");
                        }
                        false => connection
                            .rpush(tx_hash.clone())
                            .await
                            .expect("REDIS: failed to enqueue tx_hash"),
                    }
                }
                None => (),
            }
            interval.tick().await;
        }
    });
    Ok(())
}

#[cfg(test)]
pub mod tests {

    #[tokio::test]
    pub async fn unlock_tokens() {
        let response = super::unlock_tokens(
            "https://rpc.testnet.near.org".to_string(),
            "arseniyrest.testnet".to_string(),
            near_client::read_private_key::read_private_key_from_file(
                "/home/arseniyk/.near-credentials/testnet/arseniyrest.testnet.json",
            ),
            "transfer.spectrebridge.testnet".to_string(),
            spectre_bridge_common::Proof::default(),
            909090,
            300_000_000_000_000,
        )
        .await;
        assert_eq!(response.as_success().is_some(), true);
    }
}
