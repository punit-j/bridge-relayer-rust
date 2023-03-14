use crate::async_redis_wrapper::{
    AsyncRedisWrapper, PendingTransactionData, TxData, PENDING_TRANSACTIONS,
};
use crate::prometheus_metrics::{FAIL_TRANSACTIONS_COUNT, SUCCESS_TRANSACTIONS_COUNT};
use crate::{
    errors::CustomError,
    ethereum::{transactions::TransactionStatus, RainbowBridgeEthereumClient},
};
use redis::{AsyncCommands, RedisResult};
use std::{collections::HashMap, str::FromStr};
use uint::rustc_hex::ToHex;
use web3::types::H256;

macro_rules! info {
    ($($arg:tt)+) => { tracing::info!(target: crate::logs::PENDING_TRANSACTION_TARGET, $($arg)+) }
}

macro_rules! error {
    ($($arg:tt)+) => { tracing::error!(target: crate::logs::PENDING_TRANSACTION_TARGET, $($arg)+) }
}

#[allow(clippy::needless_lifetimes)]
pub async fn run<'a>(
    eth_rpc_url: url::Url,
    rainbow_bridge_index_js_path: String,
    mut redis: AsyncRedisWrapper,
    rpc_timeout_secs: u64,
) {
    let rb_index = rainbow_bridge_index_js_path.as_str();
    let eth_client =
        RainbowBridgeEthereumClient::new(eth_rpc_url, rb_index, rpc_timeout_secs).unwrap();

    // transaction hash and last processed time
    let mut pending_transactions = HashMap::<H256, PendingTransactionData>::new();

    loop {
        // fill the pending_transactions
        let mut iter: redis::AsyncIter<(String, String)> =
            redis.connection.hscan(PENDING_TRANSACTIONS).await.unwrap();

        while let Some(pair) = iter.next_item().await {
            let hash = H256::from_str(pair.0.as_str()).expect("Unable to parse tx hash");
            let data = serde_json::from_str::<PendingTransactionData>(pair.1.as_str()).unwrap();

            if let std::collections::hash_map::Entry::Vacant(e) = pending_transactions.entry(hash) {
                e.insert(data);
                info!("New pending transaction: {:#?}", hash);
            }
        }

        // process the pending_transactions
        let mut txs_to_remove: Vec<H256> = Vec::new();
        for (key, tx_data) in pending_transactions.iter_mut() {
            // remove and skip if transaction is already processing
            let key_hex = key.as_bytes().to_hex::<String>();
            if redis.get_tx_data(key_hex).await.is_ok() {
                txs_to_remove.push(*key);
            } else {
                let res = handle_one_tx(key, tx_data, &eth_client, &mut txs_to_remove, &mut redis);
                if let Err(err) = res.await {
                    error!("{}", err);
                }
            }
        }

        for item in txs_to_remove {
            let item_hex = item.as_bytes().to_hex::<String>();
            let res: RedisResult<()> = redis.connection.hdel(PENDING_TRANSACTIONS, item_hex).await;
            if let Err(error) = res {
                error!("{}", CustomError::FailedUnstorePendingTx(error));
            }
            pending_transactions.remove(&item);
        }

        tokio::time::sleep(core::time::Duration::from_secs(1)).await;
    }
}

async fn handle_one_tx(
    key: &H256,
    tx_data: &mut PendingTransactionData,
    eth_client: &RainbowBridgeEthereumClient<'_>,
    transactions_to_remove: &mut Vec<H256>,
    redis: &mut AsyncRedisWrapper,
) -> Result<(), CustomError> {
    let status = eth_client.transaction_status(*key).await;
    let status = status.map_err(|err| CustomError::FailedFetchTxStatus(err))?;
    match status {
        TransactionStatus::Pending => {
            // update the timestamp
            tx_data.timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }
        TransactionStatus::Failure(_block_number) => {
            FAIL_TRANSACTIONS_COUNT.inc_by(1);
            transactions_to_remove.push(*key);
            return Err(CustomError::FailedTxStatus(format!("{:?}", key)));
        }
        TransactionStatus::Success(block_number) => {
            let proof = eth_client.get_proof(key).await;
            let proof = proof.map_err(|err| CustomError::FailedFetchProof(err.to_string()))?;
            let data = TxData {
                block: u64::try_from(block_number).unwrap(),
                proof,
                nonce: tx_data.nonce,
            };
            let hex_key = key.as_bytes().to_hex::<String>();
            redis.store_tx(hex_key, data).await.unwrap();
            SUCCESS_TRANSACTIONS_COUNT.inc_by(1);
            transactions_to_remove.push(*key);
        }
    }
    Ok(())
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
