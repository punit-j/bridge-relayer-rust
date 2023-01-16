use crate::{async_redis_wrapper, ethereum};

use std::str::FromStr;
use redis::AsyncCommands;
use uint::rustc_hex::ToHex;

#[allow(clippy::needless_lifetimes)]
pub async fn run<'a>(
    eth_rpc_url: url::Url,
    eth_contract_address: web3::types::Address,
    eth_contract_abi: String,
    eth_keypair: web3::signing::SecretKeyRef<'a>,
    rainbow_bridge_index_js_path: String,
    mut redis: crate::async_redis_wrapper::AsyncRedisWrapper,
    _delay_request_status_sec: u64,
) {
    let eth_client = ethereum::RainbowBridgeEthereumClient::new(
        eth_rpc_url.as_str(),
        rainbow_bridge_index_js_path.as_str(),
        eth_contract_address,
        eth_contract_abi.as_bytes(),
        eth_keypair,
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
                println!("New pending transaction: {:#?}", hash)
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
                            ethereum::transactions::TransactionStatus::Pengind => {
                                // update the timestamp
                                tx_data.timestamp = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs();
                            }
                            ethereum::transactions::TransactionStatus::Failure(_block_number) => {
                                eprintln!(
                                    "{}",
                                    crate::errors::CustomError::FailedTxStatus(format!(
                                        "{:?}",
                                        key
                                    ))
                                );
                                transactions_to_remove.push(*key);
                            }
                            ethereum::transactions::TransactionStatus::Sucess(block_number) => {
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
                                        eprintln!(
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
                        eprintln!("{}", crate::errors::CustomError::FailedFetchTxStatus(error))
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
                eprintln!(
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
    use crate::pending_transactions_worker::run;
    use eth_client::test_utils::{
        get_eth_erc20_fast_bridge_contract_abi, get_eth_erc20_fast_bridge_proxy_contract_address,
        get_eth_rpc_url, get_relay_eth_key,
    };
    use crate::test_utils::{get_settings, remove_all, get_rb_index_path_str};
    use redis::AsyncCommands;
    use tokio::time::timeout;

    #[tokio::test]
    async fn smoke_pending_transactions_worker_test() {
        let settings = std::sync::Arc::new(std::sync::Mutex::new(get_settings()));
        let mut redis = AsyncRedisWrapper::connect(settings.clone()).await;
        let eth_key = get_relay_eth_key();

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
            get_eth_erc20_fast_bridge_proxy_contract_address(),
            get_eth_erc20_fast_bridge_contract_abi().await,
            web3::signing::SecretKeyRef::from(&eth_key),
            get_rb_index_path_str(),
            redis.clone(),
            0,
        );

        let timeout_duration = std::time::Duration::from_secs(10);
        let _result = timeout(timeout_duration, worker).await;

        let transactions: Vec<String> = redis.connection.hkeys(TRANSACTIONS).await.unwrap();
        assert_eq!(transactions.len(), 1);
    }
}
