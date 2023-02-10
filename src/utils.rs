use crate::async_redis_wrapper::AsyncRedisWrapper;
use crate::config::SafeSettings;
use crate::logs::EVENT_PROCESSOR_TARGET;
use crate::{config::Settings, pending_transactions_worker};
use url::Url;

pub async fn request_interval(seconds: u64) -> tokio::time::Interval {
    tokio::time::interval_at(
        tokio::time::Instant::now() + tokio::time::Duration::from_secs(seconds),
        tokio::time::Duration::from_secs(seconds),
    )
}

pub async fn get_tx_count(
    redis: &mut AsyncRedisWrapper,
    rpc_url: Url,
    relay_eth_address: web3::types::Address,
) -> Result<web3::types::U256, crate::errors::CustomError> {
    let transaction_count = redis
        .get_transaction_count()
        .await
        .unwrap_or(Some(0.into()))
        .unwrap_or(0.into());

    let transaction_count_rpc =
        eth_client::methods::get_transaction_count(rpc_url.as_str(), relay_eth_address)
            .await
            .map_err(|e| crate::errors::CustomError::FailedGetTxCount(e))?;

    Ok(std::cmp::max(transaction_count, transaction_count_rpc))
}

pub async fn build_pending_transactions_worker(
    settings: Settings,
    eth_keypair: std::sync::Arc<secp256k1::SecretKey>,
    redis: crate::async_redis_wrapper::AsyncRedisWrapper,
    eth_contract_abi: std::sync::Arc<String>,
    eth_contract_address: std::sync::Arc<web3::types::Address>,
) {
    pending_transactions_worker::run(
        settings.eth.rpc_url,
        *eth_contract_address,
        eth_contract_abi.as_ref().clone(),
        web3::signing::SecretKeyRef::from(eth_keypair.as_ref()),
        settings.eth.rainbow_bridge_index_js_path,
        redis,
        settings.rpc_timeout_secs,
    )
    .await
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
            tracing::info!(
                target: EVENT_PROCESSOR_TARGET,
                "Process event: {}",
                serde_json::to_string(&event).unwrap_or(format!("{:?}", event))
            );

            if let fast_bridge_common::Event::FastBridgeInitTransferEvent {
                nonce,
                sender_id,
                transfer_message,
            } = event
            {
                let res = crate::event_processor::process_transfer_event(
                    nonce,
                    sender_id,
                    transfer_message,
                    &settings,
                    &mut redis,
                    *eth_contract_address,
                    eth_keypair.clone(),
                    eth_contract_abi.clone(),
                    near_relay_account_id.clone(),
                )
                .await;

                if let Err(error) = res {
                    tracing::error!(
                        target: EVENT_PROCESSOR_TARGET,
                        "Failed to process tx with nonce {}, err: {}",
                        nonce.0,
                        error
                    );
                }
            }
        }
    }
}
