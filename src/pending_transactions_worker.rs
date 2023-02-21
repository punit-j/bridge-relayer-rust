use crate::logs::PENDING_TRANSACTION_TARGET;
use crate::{async_redis_wrapper, ethereum};
use redis::AsyncCommands;
use std::str::FromStr;
use uint::rustc_hex::ToHex;

#[allow(clippy::needless_lifetimes)]
pub async fn run<'a>(
    eth_rpc_url: url::Url,
    rainbow_bridge_index_js_path: String,
    mut redis: crate::async_redis_wrapper::AsyncRedisWrapper,
    rpc_timeout_secs: u64,
) {
    let eth_client = ethereum::RainbowBridgeEthereumClient::new(
        eth_rpc_url,
        rainbow_bridge_index_js_path.as_str(),
        rpc_timeout_secs,
    )
    .unwrap();

    // transaction hash and last processed time
    let mut pending_transactions: std::collections::HashMap<
        web3::types::H256,
        async_redis_wrapper::PendingTransactionData,
    > = std::collections::HashMap::new();

    loop {
        // fill the pending_transactions
        let mut iter: redis::AsyncIter<(String, String)> = redis
            .connection
            .hscan(async_redis_wrapper::PENDING_TRANSACTIONS)
            .await
            .unwrap();
        while let Some(pair) = iter.next_item().await {
            let hash =
                web3::types::H256::from_str(pair.0.as_str()).expect("Unable to parse tx hash");
            let data = serde_json::from_str::<async_redis_wrapper::PendingTransactionData>(
                pair.1.as_str(),
            )
            .unwrap();

            if let std::collections::hash_map::Entry::Vacant(e) = pending_transactions.entry(hash) {
                e.insert(data);
                tracing::info!(
                    target: PENDING_TRANSACTION_TARGET,
                    "New pending transaction: {:#?}",
                    hash
                )
            }
        }

        // process the pending_transactions
        let mut transactions_to_remove: Vec<web3::types::H256> = Vec::new();
        for (key, tx_data) in pending_transactions.iter_mut() {
            // remove and skip if transaction is already processing
            if redis
                .get_tx_data(key.as_bytes().to_hex::<String>())
                .await
                .is_ok()
            {
                transactions_to_remove.push(*key);
            } else {
                match eth_client.transaction_status(*key).await {
                    Ok(status) => {
                        match status {
                            ethereum::transactions::TransactionStatus::Pending => {
                                // update the timestamp
                                tx_data.timestamp = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs();
                            }
                            ethereum::transactions::TransactionStatus::Failure(_block_number) => {
                                tracing::error!(
                                    target: PENDING_TRANSACTION_TARGET,
                                    "{}",
                                    crate::errors::CustomError::FailedTxStatus(format!(
                                        "{:?}",
                                        key
                                    ))
                                );
                                transactions_to_remove.push(*key);
                            }
                            ethereum::transactions::TransactionStatus::Success(block_number) => {
                                let proof = eth_client.get_proof(key).await;
                                match proof {
                                    Ok(proof) => {
                                        let data = async_redis_wrapper::TxData {
                                            block: u64::try_from(block_number).unwrap(),
                                            proof,
                                            nonce: tx_data.nonce,
                                        };
                                        redis
                                            .store_tx(key.as_bytes().to_hex::<String>(), data)
                                            .await
                                            .unwrap();
                                        transactions_to_remove.push(*key);
                                    }
                                    Err(error) => {
                                        tracing::error!(
                                            target: PENDING_TRANSACTION_TARGET,
                                            "{}",
                                            crate::errors::CustomError::FailedFetchProof(
                                                error.to_string()
                                            )
                                        )
                                    }
                                }
                            }
                        }
                    }
                    Err(error) => {
                        tracing::error!(
                            target: PENDING_TRANSACTION_TARGET,
                            "{}",
                            crate::errors::CustomError::FailedFetchTxStatus(error)
                        )
                    }
                }
            }
        }

        for item in transactions_to_remove {
            let res: redis::RedisResult<()> = redis
                .connection
                .hdel(
                    async_redis_wrapper::PENDING_TRANSACTIONS,
                    item.as_bytes().to_hex::<String>(),
                )
                .await;
            if let Err(error) = res {
                tracing::error!(
                    target: PENDING_TRANSACTION_TARGET,
                    "{}",
                    crate::errors::CustomError::FailedUnstorePendingTx(error)
                );
            }
            pending_transactions.remove(&item);
        }

        tokio::time::sleep(core::time::Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
pub mod tests {
    use crate::async_redis_wrapper;
    use crate::async_redis_wrapper::{AsyncRedisWrapper, TRANSACTIONS};
    use crate::logs::init_logger;
    use crate::pending_transactions_worker::run;
    use crate::test_utils::{get_rb_index_path_str, get_settings, remove_all};
    use eth_client::test_utils::get_eth_rpc_url;
    use redis::AsyncCommands;
    use tokio::time::timeout;

    #[tokio::test]
    async fn smoke_pending_transactions_worker_test() {
        init_logger();

        let settings = std::sync::Arc::new(tokio::sync::Mutex::new(get_settings()));
        let mut redis = AsyncRedisWrapper::connect(&settings.lock().await.redis).await;

        remove_all(redis.clone(), async_redis_wrapper::PENDING_TRANSACTIONS).await;
        remove_all(redis.clone(), async_redis_wrapper::TRANSACTIONS).await;

        let d = crate::async_redis_wrapper::PendingTransactionData {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            nonce: 605226883 as u128,
        };

        let _res: () = redis
            .connection
            .hset(
                async_redis_wrapper::PENDING_TRANSACTIONS,
                "ac8b251f1b4eeaacbdfbc2fa1711c201fdb628f5670680997194f17bc9de1baf",
                serde_json::to_string(&d).unwrap(),
            )
            .await
            .unwrap();

        let worker = run(
            get_eth_rpc_url(),
            get_rb_index_path_str(),
            redis.clone(),
            30,
        );

        let timeout_duration = std::time::Duration::from_secs(10);
        let _result = timeout(timeout_duration, worker).await;

        let transactions: Vec<String> = redis.connection.hkeys(TRANSACTIONS).await.unwrap();
        assert_eq!(transactions.len(), 1);
    }
}
