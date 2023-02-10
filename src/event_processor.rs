use crate::async_redis_wrapper::{self, AsyncRedisWrapper};
use crate::config::Settings;
use crate::errors::CustomError;
use crate::logs::EVENT_PROCESSOR_TARGET;
use crate::utils::get_tx_count;
use near_sdk::AccountId;
use redis::AsyncCommands;
use uint::rustc_hex::ToHex;
use web3::signing::*;

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

    tracing::info!(
        target: EVENT_PROCESSOR_TARGET,
        "Execute transfer on eth with nonce {:?}",
        nonce
    );

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

    tracing::info!(
        target: EVENT_PROCESSOR_TARGET,
        "New eth transaction: {:#?}",
        tx_hash
    );

    let pending_transaction_data = crate::async_redis_wrapper::PendingTransactionData {
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
    res.map_err(|e| crate::errors::CustomError::FailedStorePendingTx(e))?;

    transaction_count += 1.into();
    redis
        .set_transaction_count(transaction_count)
        .await
        .map_err(|e| crate::errors::CustomError::FailedSetTxCount(e))?;

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use crate::async_redis_wrapper::{AsyncRedisWrapper, PENDING_TRANSACTIONS};
    use crate::event_processor::process_transfer_event;
    use crate::logs::init_logger;
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
        let valid_till = 0;
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
        .await;

        tokio::time::sleep(Duration::from_secs(60)).await;

        let new_pending_transactions: Vec<String> =
            redis.connection.hkeys(PENDING_TRANSACTIONS).await.unwrap();
        assert_eq!(
            pending_transactions.len() + 1,
            new_pending_transactions.len()
        );
    }
}
