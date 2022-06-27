async fn unlock_tokens(
    server_addr: url::Url,
    account: near_crypto::InMemorySigner,
    contract_account_id: String,
    proof: spectre_bridge_common::Proof,
    nonce: u128,
    gas: u64,
) -> Result<near_primitives::views::FinalExecutionStatus, String> {
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
    .await;
    match response {
        Ok(result) => Ok(result.status),
        Err(error) => {
            return Err(format!(
                "Failed to fetch response by calling lp_unlock contract method: {}",
                error
            )
            .into())
        }
    }
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
                settings.lock().unwrap().clone().unlock_tokens_worker;
            let mut interval =
                crate::utils::request_interval(unlock_tokens_worker_settings.request_interval_secs)
                    .await;
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval.tick().await;
            let tx_hashes_queue = connection
                .get_tx_hashes(crate::async_redis_wrapper::TRANSACTIONS)
                .await;
            match tx_hashes_queue {
                Ok(mut queue) => {
                    while !queue.is_empty() {
                        match queue.first() {
                            Some(tx_hash) => {
                                let last_block_number =
                                    storage.lock().unwrap().clone().last_block_number;
                                let tx_data = connection.get_tx_data(tx_hash.clone()).await;
                                match tx_data {
                                    Ok(data) => {
                                        match data.block + unlock_tokens_worker_settings.some_blocks_number
                                            <= last_block_number
                                        {
                                            true => {
                                                let tx_execution_status = crate::unlock_tokens::unlock_tokens(
                                                    unlock_tokens_worker_settings.server_addr.clone(),
                                                    account.clone(),
                                                    unlock_tokens_worker_settings.contract_account_id.clone(),
                                                    data.proof,
                                                    data.nonce,
                                                    gas,
                                                )
                                                .await;
                                                if let Ok(
                                                    near_primitives::views::FinalExecutionStatus::SuccessValue(
                                                        _,
                                                    ),
                                                ) = tx_execution_status
                                                {
                                                    let unstore_tx_status =
                                                        connection.unstore_tx(tx_hash.to_string()).await;
                                                    match unstore_tx_status {
                                                        Ok(_) => {
                                                            queue.remove(0);
                                                        }
                                                        Err(error) => eprintln!(
                                                            "REDIS: Failed to unstore transaction: {}",
                                                            error
                                                        ),
                                                    }
                                                } else {
                                                    eprintln!(
                                                        "Failed to unlock tokens: {}",
                                                        tx_execution_status.unwrap_err()
                                                    )
                                                }
                                            }
                                            false => {
                                                let queue_len = queue.len();
                                                queue[0..queue_len]
                                                .rotate_left(1)
                                            },
                                        }
                                    }
                                    Err(error) => eprintln!(
                                        "REDIS: Failed to get transaction data by hash from set: {}",
                                        error
                                    ),
                                }
                            }
                            None => (),
                        }
                    }
                }
                Err(error) => eprintln!("REDIS: Failed to get queue of tx_hashes: {}", error),
            }
        }
    });
    Ok(())
}
