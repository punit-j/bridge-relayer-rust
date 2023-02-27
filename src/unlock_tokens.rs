use crate::{
    async_redis_wrapper::AsyncRedisWrapper, config::SafeSettings, errors::CustomError,
    last_block::SafeStorage,
};
use near_primitives::{
    hash::CryptoHash,
    views::{ExecutionStatusView::Failure, FinalExecutionStatus},
};

async fn unlock_tokens(
    server_addr: url::Url,
    account: near_crypto::InMemorySigner,
    contract_account_id: String,
    proof: fast_bridge_common::Proof,
    nonce: u128,
    gas: u64,
) -> Result<(FinalExecutionStatus, CryptoHash), CustomError> {
    tracing::info!("Start lp unlock for token with nonce={}", nonce);
    let result = near_client::methods::change(
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
    .map_err(|err| CustomError::FailedExecuteUnlockTokens(err.to_string()))?;

    for receipt_outcome in result.receipts_outcome {
        if let Failure(tx_error) = receipt_outcome.outcome.status {
            return Ok((
                FinalExecutionStatus::Failure(tx_error),
                result.transaction.hash,
            ));
        }
    }
    Ok((result.status, result.transaction.hash))
}

async fn handle_one_tx(
    account: near_crypto::InMemorySigner,
    gas: u64,
    unlock_tokens_worker_settings: crate::config::UnlockTokensWorkerSettings,
    tx_hash: String,
    storage: SafeStorage,
    mut redis: AsyncRedisWrapper,
) -> Result<(), String> {
    tracing::info!(
        "Start processing transaction for lp unlock (tx_hash={})",
        tx_hash
    );

    let eth_last_block_number_on_near = storage.lock().await.eth_last_block_number_on_near;
    let tx_data = redis
        .get_tx_data(tx_hash.clone())
        .await
        .map_err(|err| format!("{}", CustomError::FailedGetTxData(err)))?;
    let unlock_tokens_execution_condition = tx_data.block
        + unlock_tokens_worker_settings.blocks_for_tx_finalization
        <= eth_last_block_number_on_near;

    if !unlock_tokens_execution_condition {
        tracing::info!(
            "Skip tx(nonce={}, tx_hash={}); \n\
                          Current last ETH block on NEAR = {}, \n\
                          ETH block with tx = {}, \n\
                          Waiting for block = {}",
            tx_data.nonce,
            tx_hash,
            eth_last_block_number_on_near,
            tx_data.block,
            tx_data.block + unlock_tokens_worker_settings.blocks_for_tx_finalization
        );
        return Ok(());
    }

    let (tx_execution_status, near_tx_hash) = unlock_tokens(
        unlock_tokens_worker_settings.server_addr.clone(),
        account.clone(),
        unlock_tokens_worker_settings.contract_account_id.clone(),
        tx_data.proof,
        tx_data.nonce,
        gas,
    )
    .await
    .map_err(|err| format!("{}", err))?;

    match tx_execution_status {
        FinalExecutionStatus::NotStarted | FinalExecutionStatus::Started => {
            return Err(format!(
                "Tx status (nonce: {}): {:?}; NEAR tx_hash: {}",
                tx_data.nonce, tx_execution_status, near_tx_hash
            ));
        }
        FinalExecutionStatus::Failure(_) => {
            unstore_tx(&mut redis, &tx_hash).await;
            return Err(format!(
                "Failed transaction (nonce: {}): {:?}; NEAR tx_hash: {}",
                tx_data.nonce, tx_execution_status, near_tx_hash
            ));
        }
        FinalExecutionStatus::SuccessValue(_) => {
            unstore_tx(&mut redis, &tx_hash).await;
            tracing::info!(
                "Tokens unlocked (nonce: {}). NEAR tx_hash = {}",
                tx_data.nonce,
                near_tx_hash
            );
        }
    }
    Ok(())
}

async fn unstore_tx(connection: &mut AsyncRedisWrapper, tx_hash: &String) {
    let unstore_tx_status = connection.unstore_tx(tx_hash.to_string()).await;
    if let Err(error) = unstore_tx_status {
        tracing::error!("{}", CustomError::FailedUnstoreTransaction(error))
    }
}

pub async fn unlock_tokens_worker(
    account: near_crypto::InMemorySigner,
    gas: u64,
    settings: SafeSettings,
    storage: SafeStorage,
    mut redis: AsyncRedisWrapper,
) {
    loop {
        let unlock_tokens_settings = settings.lock().await.unlock_tokens_worker.clone();
        let interval_secs = unlock_tokens_settings.request_interval_secs;
        tracing::trace!("unlock_tokens_worker: sleep for {} secs", interval_secs);

        let mut interval = crate::utils::request_interval(interval_secs).await;
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;

        match redis.get_tx_hashes().await {
            Ok(queue) => {
                for tx_hash in queue {
                    let res = handle_one_tx(
                        account.clone(),
                        gas,
                        unlock_tokens_settings.clone(),
                        tx_hash,
                        storage.clone(),
                        redis.clone(),
                    )
                    .await;
                    if let Err(err) = res {
                        tracing::error!(err);
                    }
                }
            }
            Err(error) => tracing::error!("{}", CustomError::FailedGetTxHashesQueue(error)),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::async_redis_wrapper::{AsyncRedisWrapper, TRANSACTIONS};
    use crate::config::default_rpc_timeout_secs;
    use crate::last_block::Storage;
    use crate::logs::init_logger;
    use crate::test_utils::{get_rb_index_path_str, get_settings, remove_all};
    use crate::unlock_tokens::unlock_tokens_worker;
    use crate::{async_redis_wrapper, ethereum};
    use eth_client::test_utils::get_eth_rpc_url;
    use near_client::test_utils::get_near_signer;
    use std::str::FromStr;
    use tokio::time::timeout;

    async fn add_transaction(mut redis: AsyncRedisWrapper) {
        remove_all(redis.clone(), TRANSACTIONS).await;
        let eth_rpc_url = get_eth_rpc_url();
        let rb_index_path_str = get_rb_index_path_str();

        let eth_client = ethereum::RainbowBridgeEthereumClient::new(
            eth_rpc_url,
            &rb_index_path_str,
            default_rpc_timeout_secs(),
        )
        .unwrap();

        let tx_hash = web3::types::H256::from_str(
            "ac8b251f1b4eeaacbdfbc2fa1711c201fdb628f5670680997194f17bc9de1baf",
        )
        .unwrap();
        let proof = eth_client.get_proof(&tx_hash).await.unwrap();

        let data = async_redis_wrapper::TxData {
            block: 8249153 as u64,
            proof,
            nonce: 605226883 as u128,
        };
        redis
            .store_tx(
                "ac8b251f1b4eeaacbdfbc2fa1711c201fdb628f5670680997194f17bc9de1baf".to_string(),
                data,
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn smoke_unlock_tokens_worker_test() {
        init_logger();

        let signer = get_near_signer();

        let settings = std::sync::Arc::new(tokio::sync::Mutex::new(get_settings()));
        let redis = AsyncRedisWrapper::connect(&settings.lock().await.redis).await;
        add_transaction(redis.clone()).await;

        let storage = std::sync::Arc::new(tokio::sync::Mutex::new(Storage::new()));
        storage.lock().await.eth_last_block_number_on_near = 8249163;

        let worker = unlock_tokens_worker(
            signer,
            230_000_000_000_000u64,
            settings.clone(),
            storage.clone(),
            redis,
        );

        let timeout_duration = std::time::Duration::from_secs(10);
        let _result = timeout(timeout_duration, worker).await;
    }
}
