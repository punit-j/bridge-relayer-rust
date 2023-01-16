use near_primitives::views::ExecutionStatusView::Failure;
use crate::config::Settings;

async fn unlock_tokens(
    server_addr: url::Url,
    account: near_crypto::InMemorySigner,
    contract_account_id: String,
    proof: spectre_bridge_common::Proof,
    nonce: u128,
    gas: u64,
) -> Result<near_primitives::views::FinalExecutionStatus, crate::errors::CustomError> {
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
        Ok(result) =>  {
            for receipt_outcome in result.receipts_outcome {
                if let Failure(tx_error) = receipt_outcome.outcome.status {
                    return Ok(near_primitives::views::FinalExecutionStatus::Failure(tx_error))
                }
            }

            Ok(result.status)
        },
        Err(error) => Err(crate::errors::CustomError::FailedExecuteUnlockTokens(
            error.to_string(),
        )),
    }
}

async fn transactions_traversal(
    account: near_crypto::InMemorySigner,
    gas: u64,
    unlock_tokens_worker_settings: crate::config::UnlockTokensWorkerSettings,
    tx_hashes_queue: Vec<String>,
    storage: std::sync::Arc<std::sync::Mutex<crate::last_block::Storage>>,
    redis: std::sync::Arc<std::sync::Mutex<crate::async_redis_wrapper::AsyncRedisWrapper>>,
) {
    let mut connection = redis.lock().unwrap().clone();
    for tx_hash in tx_hashes_queue {
        println!("Handling transaction {}", tx_hash);

        let eth_last_block_number_on_near = storage
            .lock()
            .unwrap()
            .clone()
            .eth_last_block_number_on_near;
        let tx_data = connection.get_tx_data(tx_hash.clone()).await;
        match tx_data {
            Ok(data) => {
                let unlock_tokens_execution_condition = data.block
                    + unlock_tokens_worker_settings.blocks_for_tx_finalization
                    <= eth_last_block_number_on_near;
                if unlock_tokens_execution_condition {
                    let tx_execution_status = crate::unlock_tokens::unlock_tokens(
                        unlock_tokens_worker_settings.server_addr.clone(),
                        account.clone(),
                        unlock_tokens_worker_settings.contract_account_id.clone(),
                        data.proof,
                        data.nonce,
                        gas,
                    )
                    .await;

                    match tx_execution_status {
                        Ok(tx_execution_status) => match tx_execution_status {
                            near_primitives::views::FinalExecutionStatus::NotStarted
                            | near_primitives::views::FinalExecutionStatus::Started => {
                                eprintln!("{:?}", tx_execution_status)
                            }
                            near_primitives::views::FinalExecutionStatus::Failure(_) => {
                                eprintln!("Failed transaction: {:?}", tx_execution_status);
                                unstore_tx(&mut connection, &tx_hash, data.nonce).await;
                            }
                            near_primitives::views::FinalExecutionStatus::SuccessValue(_) => {
                                unstore_tx(&mut connection, &tx_hash, data.nonce).await;
                            }
                        },
                        Err(err) => eprintln!("{}", err),
                    }
                } else {
                    println!("Skip tx; Current eth last block = {}, proof block = {}, blocks for tx finalization = {}", eth_last_block_number_on_near, data.block, unlock_tokens_worker_settings.blocks_for_tx_finalization);
                    continue;
                }
            }
            Err(error) => eprintln!("{}", crate::errors::CustomError::FailedGetTxData(error)),
        }
    }
}

async fn unstore_tx(
    connection: &mut crate::async_redis_wrapper::AsyncRedisWrapper,
    tx_hash: &String,
    nonce: u128,
) {
    let unstore_tx_status = connection.unstore_tx(tx_hash.to_string()).await;
    match unstore_tx_status {
        Ok(_) => {
            println!("Tokens successfully unlocked (nonce: {})", nonce)
        }
        Err(error) => {
            eprintln!(
                "{}",
                crate::errors::CustomError::FailedUnstoreTransaction(error)
            )
        }
    }
}

pub async fn unlock_tokens_worker(
    account: near_crypto::InMemorySigner,
    gas: u64,
    settings: std::sync::Arc<std::sync::Mutex<Settings>>,
    storage: std::sync::Arc<std::sync::Mutex<crate::last_block::Storage>>,
    redis: std::sync::Arc<std::sync::Mutex<crate::async_redis_wrapper::AsyncRedisWrapper>>,
) {
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
            let tx_hashes_queue = connection.get_tx_hashes().await;
            match tx_hashes_queue {
                Ok(queue) => {
                    transactions_traversal(
                        account.clone(),
                        gas,
                        unlock_tokens_worker_settings,
                        queue,
                        storage.clone(),
                        redis.clone(),
                    )
                    .await;
                }
                Err(error) => eprintln!(
                    "{}",
                    crate::errors::CustomError::FailedGetTxHashesQueue(error)
                ),
            }
        }
    });
}

#[cfg(test)]
pub mod tests {
    use crate::async_redis_wrapper::{AsyncRedisWrapper, TRANSACTIONS};
    use crate::last_block::Storage;
    use crate::test_utils::{get_rb_index_path_str, get_settings, remove_all};
    use crate::unlock_tokens::unlock_tokens_worker;
    use crate::{async_redis_wrapper, ethereum};
    use eth_client::test_utils::{
        get_eth_erc20_fast_bridge_contract_abi, get_eth_erc20_fast_bridge_proxy_contract_address,
        get_eth_rpc_url, get_relay_eth_key,
    };
    use near_client::test_utils::get_near_signer;
    use std::str::FromStr;
    use std::time::Duration;

    async fn add_transaction(mut redis: AsyncRedisWrapper) {
        remove_all(redis.clone(), TRANSACTIONS).await;
        let relay_eth_key = get_relay_eth_key();
        let eth_rpc_url = get_eth_rpc_url();
        let rb_index_path_str = get_rb_index_path_str();

        let eth_client = ethereum::RainbowBridgeEthereumClient::new(
            eth_rpc_url.as_str(),
            &rb_index_path_str,
            get_eth_erc20_fast_bridge_proxy_contract_address(),
            get_eth_erc20_fast_bridge_contract_abi().await.as_bytes(),
            web3::signing::SecretKeyRef::from(&relay_eth_key),
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
        let signer = get_near_signer();

        let settings = std::sync::Arc::new(std::sync::Mutex::new(get_settings()));
        let redis = AsyncRedisWrapper::connect(settings.clone()).await;
        add_transaction(redis.clone()).await;

        let arc_redis = std::sync::Arc::new(std::sync::Mutex::new(redis));

        let storage = std::sync::Arc::new(std::sync::Mutex::new(Storage::new()));
        storage.lock().unwrap().eth_last_block_number_on_near = 8249163;

        let _worker = unlock_tokens_worker(
            signer,
            230_000_000_000_000u64,
            settings.clone(),
            storage.clone(),
            arc_redis,
        )
        .await;

        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
