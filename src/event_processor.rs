use crate::async_redis_wrapper::{self, AsyncRedisWrapper};
use crate::config::{SafeSettings, Settings};
use crate::prometheus_metrics::{
    CONNECTION_ERRORS, PENDING_TRANSACTIONS_COUNT, SKIP_TRANSACTIONS_COUNT,
};
use crate::{errors::CustomError, utils::get_tx_count};
use fast_bridge_common::Event::FastBridgeInitTransferEvent;
use near_sdk::AccountId;
use redis::AsyncCommands;
use uint::rustc_hex::ToHex;
use web3::{contract::Error::Api, signing::*, Error::Rpc, Error::Transport};

macro_rules! info {
    ($($arg:tt)+) => { tracing::info!(target: crate::logs::EVENT_PROCESSOR_TARGET, $($arg)+) }
}

macro_rules! error {
    ($($arg:tt)+) => { tracing::error!(target: crate::logs::EVENT_PROCESSOR_TARGET, $($arg)+) }
}

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
) -> Result<(), CustomError> {
    let rpc_url = settings.eth.rpc_url.clone();
    let mut transaction_count =
        get_tx_count(redis, rpc_url.clone(), relay_eth_key.address()).await?;

    info!("Execute transfer on eth with nonce {:?}", nonce);

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
    .await?;

    info!("New eth transaction: {:#?}", tx_hash);

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
    PENDING_TRANSACTIONS_COUNT.inc_by(1);

    transaction_count += 1.into();
    redis
        .set_transaction_count(transaction_count)
        .await
        .map_err(|e| CustomError::FailedSetTxCount(e))?;

    Ok(())
}

pub async fn build_near_events_subscriber(
    settings: SafeSettings,
    eth_keypair: std::sync::Arc<secp256k1::SecretKey>,
    mut redis: AsyncRedisWrapper,
    eth_contract_abi: std::sync::Arc<String>,
    eth_contract_address: std::sync::Arc<web3::types::Address>,
    mut stream: tokio::sync::mpsc::Receiver<String>,
    near_relay_account_id: String,
) {
    while let Some(msg) = stream.recv().await {
        let settings = settings.lock().await.clone();

        if let Ok(event) = serde_json::from_str::<fast_bridge_common::Event>(msg.as_str()) {
            let event_str = serde_json::to_string(&event).unwrap_or(format!("{:?}", event));
            info!("Process event: {}", event_str);

            if let FastBridgeInitTransferEvent {
                nonce,
                sender_id,
                transfer_message,
            } = event
            {
                loop {
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
                    )
                    .await;

                    if let Err(error) = res {
                        if is_connection_error(&error) {
                            CONNECTION_ERRORS.inc_by(1);
                            error!("Failed to process tx with nonce {}, err: {:?}. Repeat try after 15s.", nonce.0, error);
                            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
                            continue;
                        } else {
                            SKIP_TRANSACTIONS_COUNT.inc_by(1);
                            error!(
                                "Failed to process tx with nonce {}, err: {:?}. Skip transaction.",
                                nonce.0, error
                            );
                        }
                    }

                    break;
                }
            }
        }
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

#[cfg(test)]
pub mod tests {
    use crate::async_redis_wrapper::{AsyncRedisWrapper, PENDING_TRANSACTIONS};
    use crate::event_processor::process_transfer_event;
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

    #[tokio::test]
    async fn smoke_process_transfer_event_test() {
        init_logger();

        let nonce = U128::from(rand::thread_rng().gen_range(0..1000000000));
        let valid_till = test_utils::get_valid_till();
        let transfer = TransferDataEthereum {
            token_near: get_near_token(),
            token_eth: EthAddress::from(get_eth_token()),
            amount: U128::from(1),
        };
        let fee = TransferDataNear {
            token: get_near_token(),
            amount: U128::from(1_000_000_000),
        };
        let recipient = EthAddress::from(get_recipient());

        let settings = get_settings();
        let settings = std::sync::Arc::new(tokio::sync::Mutex::new(settings));
        let mut redis = AsyncRedisWrapper::connect(&settings.lock().await.redis).await;

        let relay_eth_key = std::sync::Arc::new(get_relay_eth_key());
        let eth_erc20_fast_bridge_contract_abi =
            std::sync::Arc::new(get_eth_erc20_fast_bridge_contract_abi().await);

        let near_account = get_near_signer().account_id.to_string();

        let pending_transactions: Vec<String> =
            redis.connection.hkeys(PENDING_TRANSACTIONS).await.unwrap();

        let _res = process_transfer_event(
            nonce,
            near_account.parse().unwrap(),
            TransferMessage {
                valid_till,
                transfer,
                fee,
                recipient,
                valid_till_block_height: None,
            },
            &settings.lock().await.clone(),
            &mut redis,
            get_eth_erc20_fast_bridge_proxy_contract_address(),
            relay_eth_key.clone(),
            eth_erc20_fast_bridge_contract_abi.clone(),
            near_account,
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
