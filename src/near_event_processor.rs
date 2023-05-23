use crate::async_redis_wrapper::{self, AsyncRedisWrapper, NEW_EVENTS};
use crate::config::{SafeSettings, Settings};
use crate::prometheus_metrics::{BALANCE_ERRORS, CONNECTION_ERRORS, PENDING_TRANSACTIONS_COUNT, SKIP_TRANSACTIONS_COUNT};
use crate::{errors::CustomError, utils::get_tx_count};
use fast_bridge_common::Event::FastBridgeInitTransferEvent;
use near_sdk::AccountId;
use redis::AsyncCommands;
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;
use tracing::log::warn;
use uint::rustc_hex::ToHex;
use web3::types::{H256, U256};
use web3::{contract::Error::Api, Error::Rpc, Error::Transport, signing::Key};

macro_rules! info {
    ($($arg:tt)+) => { tracing::info!(target: crate::logs::EVENT_PROCESSOR_TARGET, $($arg)+) }
}

macro_rules! error {
    ($($arg:tt)+) => { tracing::error!(target: crate::logs::EVENT_PROCESSOR_TARGET, $($arg)+) }
}

const SLEEP_TIME_AFTER_EVENTS_PROCESS_SEC: u64 = 10;
const MAX_NEW_EVENTS_BATCH: usize = 1_000_000;

#[allow(clippy::too_many_arguments)]
pub async fn process_transfer_event(
    nonce: near_sdk::json_types::U128,
    sender_id: AccountId,
    transfer_message: fast_bridge_common::TransferMessage,
    settings: &Settings,
    redis: &mut AsyncRedisWrapper,
    eth_erc20_fast_bridge_proxy_contract_address: web3::types::Address,
    relay_eth_key: std::sync::Arc<secp256k1::SecretKey>,
    eth_erc20_fast_bridge_contract_abi: std::sync::Arc<String>,
    near_relay_account_id: String,
    pending_events: &mut HashMap<u128, H256>,
) -> Result<(), CustomError> {
    let rpc_url = settings.eth.rpc_url.clone();
    let transaction_count = get_tx_count(redis, rpc_url.clone(), relay_eth_key.address()).await?;

    info!("Execute transfer on eth with nonce {:?}", nonce);

    if pending_events.contains_key(&nonce.0) {
        let tx_hash = pending_events[&nonce.0];
        return update_pending_transactions(
            tx_hash,
            nonce,
            redis,
            transaction_count,
            pending_events,
        )
        .await;
    }

    let tx_hash = crate::transfer::execute_transfer(
        relay_eth_key.clone().as_ref(),
        fast_bridge_common::Event::FastBridgeInitTransferEvent {
            nonce,
            sender_id,
            transfer_message,
        },
        eth_erc20_fast_bridge_contract_abi.as_bytes(),
        rpc_url,
        eth_erc20_fast_bridge_proxy_contract_address,
        settings.profit_thershold,
        &settings,
        near_relay_account_id,
        transaction_count,
    )
    .await;

    match tx_hash {
        Ok(tx_hash) => {
            info!("New eth transaction: {:#?}", tx_hash);
            pending_events.insert(nonce.0, tx_hash);
            update_pending_transactions(tx_hash, nonce, redis, transaction_count, pending_events)
                .await
        }
        Err(error) => {
            if is_connection_error(&error) {
                CONNECTION_ERRORS.inc();
            } else if is_balance_error(&error) {
                BALANCE_ERRORS.inc();
            } else {
                warn!(
                    "Failed to process tx with nonce {}, err: {:?}. Skip transaction.",
                    nonce.0, error
                );
                let res: redis::RedisResult<()> = redis.remove_new_event(nonce.0).await;
                res.map_err(|e| CustomError::FailedRemoveNewEvent(e))?;
                SKIP_TRANSACTIONS_COUNT.inc();
            }
            Err(error)
        }
    }
}

pub async fn update_pending_transactions(
    tx_hash: H256,
    nonce: near_sdk::json_types::U128,
    redis: &mut AsyncRedisWrapper,
    mut transaction_count: U256,
    pending_events: &mut HashMap<u128, H256>,
) -> Result<(), CustomError> {
    let pending_transaction_data = async_redis_wrapper::PendingTransactionData {
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        nonce: u128::from(nonce),
    };

    let res: redis::RedisResult<()> = redis
        .connection
        .hset(
            async_redis_wrapper::PENDING_TRANSACTIONS,
            tx_hash.as_bytes().to_hex::<String>(),
            serde_json::to_string(&pending_transaction_data).unwrap(),
        )
        .await;
    res.map_err(|e| CustomError::FailedStorePendingTx(e))?;
    PENDING_TRANSACTIONS_COUNT.inc();

    let res: redis::RedisResult<()> = redis.remove_new_event(nonce.0).await;
    res.map_err(|e| CustomError::FailedRemoveNewEvent(e))?;

    pending_events.remove(&nonce.0);

    transaction_count += 1.into();
    redis
        .set_transaction_count(transaction_count)
        .await
        .map_err(|e| CustomError::FailedSetTxCount(e))?;

    Ok(())
}

pub async fn process_near_events_worker(
    settings: SafeSettings,
    eth_keypair: std::sync::Arc<secp256k1::SecretKey>,
    mut redis: AsyncRedisWrapper,
    eth_contract_abi: std::sync::Arc<String>,
    eth_contract_address: std::sync::Arc<web3::types::Address>,
    near_relay_account_id: String,
) {
    let mut pending_events: HashMap<u128, H256> = HashMap::new();

    loop {
        let mut iter: redis::AsyncIter<(String, String)> = match redis.connection.hscan(NEW_EVENTS).await {
            Ok(iter) => iter,
            Err(err) => {
                warn!("Error on getting new events: {:?}", err);
                sleep(Duration::from_secs(SLEEP_TIME_AFTER_EVENTS_PROCESS_SEC));
                continue;
            }
        };
        let mut new_events: Vec<fast_bridge_common::Event> = vec![];
        let settings = settings.lock().await.clone();

        loop {
            if new_events.len() >= MAX_NEW_EVENTS_BATCH {
                break;
            }

            if let Some(pair) = iter.next_item().await {
                if let Ok(event) = serde_json::from_str::<fast_bridge_common::Event>(pair.1.as_str()) {
                    new_events.push(event);
                }
            } else {
                break;
            }
        }

        for event in new_events {
            let event_str = serde_json::to_string(&event).unwrap_or(format!("{:?}", event));
            info!("Process event: {}", event_str);

            if let FastBridgeInitTransferEvent {
                nonce,
                sender_id,
                transfer_message,
            } = event
            {
                let res = process_transfer_event(
                    nonce,
                    sender_id.clone(),
                    transfer_message.clone(),
                    &settings,
                    &mut redis,
                    *eth_contract_address,
                    eth_keypair.clone(),
                    eth_contract_abi.clone(),
                    near_relay_account_id.clone(),
                    &mut pending_events,
                )
                    .await;

                if let Err(error) = res {
                    error!(
                        "Failed to process tx with nonce {}, err: {:?}.",
                        nonce.0, error
                    );
                }
            }
        }
        sleep(Duration::from_secs(SLEEP_TIME_AFTER_EVENTS_PROCESS_SEC));
    }
}

fn is_connection_error(error: &CustomError) -> bool {
    match error {
        CustomError::FailedExecuteTransferTokens(Api(Transport(_)))
        | CustomError::FailedFetchGasPrice(Api(Transport(_)))
        | CustomError::FailedEstimateGas(Api(Transport(_)))
        | CustomError::FailedGetTxCount(Transport(_))
        | CustomError::FailedGetTokenPrice(_)
        | CustomError::FailedFetchEthereumPrice(_) => true,
        CustomError::FailedExecuteTransferTokens(Api(Rpc(ref rpc_error))) => rpc_error
            .message
            .contains("replacement transaction underpriced"),
        _ => false,
    }
}

fn is_balance_error(error: &CustomError) -> bool {
    match error {
        CustomError::FailedEstimateGas(Api(Rpc(ref rpc_error)))
        | CustomError::FailedExecuteTransferTokens(Api(Rpc(ref rpc_error))) => {
            if rpc_error.message.contains("insufficient allowance") ||
                rpc_error.message.contains("transfer amount exceeds balance") ||
                rpc_error.message.contains("insufficient funds for gas * price + value") {
                true
            } else {
                false
            }
        },
        _ => false,
    }
}

#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;
    use crate::async_redis_wrapper::{AsyncRedisWrapper, PENDING_TRANSACTIONS};
    use crate::near_event_processor::process_transfer_event;
    use crate::logs::init_logger;
    use crate::test_utils;
    use crate::test_utils::get_settings;
    use eth_client::test_utils::{
        get_eth_erc20_fast_bridge_contract_abi, get_eth_erc20_fast_bridge_proxy_contract_address,
        get_eth_token, get_recipient, get_relay_eth_key,
    };
    use fast_bridge_common::{EthAddress, TransferDataEthereum, TransferDataNear, TransferMessage};
    use near_client::test_utils::{get_near_signer, get_near_token};
    use near_sdk::json_types::U128;
    use rand::Rng;
    use redis::AsyncCommands;
    use std::time::Duration;
    use web3::types::H256;

    #[tokio::test]
    async fn smoke_process_transfer_event_test() {
        init_logger();

        let nonce = U128::from(rand::thread_rng().gen_range(0..1000000000));
        let valid_till = test_utils::get_valid_till();
        let transfer = TransferDataEthereum {
            token_near: get_near_token(),
            token_eth: EthAddress(get_eth_token().into()),
            amount: U128::from(1),
        };
        let fee = TransferDataNear {
            token: get_near_token(),
            amount: U128::from(1_000_000_000),
        };
        let recipient = EthAddress(get_recipient().into());

        let settings = get_settings();
        let settings = std::sync::Arc::new(tokio::sync::Mutex::new(settings));
        let mut redis = AsyncRedisWrapper::connect(&settings.lock().await.redis).await;

        let relay_eth_key = std::sync::Arc::new(get_relay_eth_key());
        let eth_erc20_fast_bridge_contract_abi =
            std::sync::Arc::new(get_eth_erc20_fast_bridge_contract_abi().await);

        let near_account = get_near_signer().account_id.to_string();

        let pending_transactions: Vec<String> =
            redis.connection.hkeys(PENDING_TRANSACTIONS).await.unwrap();

        let mut pending_events: HashMap<u128, H256> = HashMap::new();

        let _res = process_transfer_event(
            nonce,
            near_account.parse().unwrap(),
            TransferMessage {
                valid_till,
                transfer,
                fee,
                recipient,
                valid_till_block_height: Some(100_000_000_000),
                aurora_sender: None,
            },
            &settings.lock().await.clone(),
            &mut redis,
            get_eth_erc20_fast_bridge_proxy_contract_address(),
            relay_eth_key.clone(),
            eth_erc20_fast_bridge_contract_abi.clone(),
            near_account,
            &mut pending_events
        )
        .await
        .unwrap();

        tokio::time::sleep(Duration::from_secs(60)).await;

        let new_pending_transactions: Vec<String> =
            redis.connection.hkeys(PENDING_TRANSACTIONS).await.unwrap();

        assert_eq!(
            pending_transactions.len() + 1,
            new_pending_transactions.len()
        );
    }
}
