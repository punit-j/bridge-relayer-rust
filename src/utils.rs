use crate::logs::EVENT_PROCESSOR_TARGET;
use crate::{config::Settings, pending_transactions_worker, async_redis_wrapper::SafeAsyncRedisWrapper};
use tokio::task::JoinHandle;
use url::Url;
use web3::types::Res;

pub async fn request_interval(seconds: u64) -> tokio::time::Interval {
    tokio::time::interval_at(
        tokio::time::Instant::now() + tokio::time::Duration::from_secs(seconds),
        tokio::time::Duration::from_secs(seconds),
    )
}

pub async fn get_tx_count(redis: SafeAsyncRedisWrapper,
                          rpc_url: Url,
                          relay_eth_address: web3::types::Address
) -> Result<web3::types::U256, crate::errors::CustomError> {
    let mut transaction_count = redis
        .lock()
        .clone()
        .get_mut()
        .get_transaction_count()
        .await
        .unwrap_or(0.into());

    let transaction_count_rpc =
        eth_client::methods::get_transaction_count(rpc_url.as_str(), relay_eth_address)
            .await
            .map_err(|e| crate::errors::CustomError::FailedGetTxCount(e))?;

    Ok(std::cmp::max(transaction_count, transaction_count_rpc))
}

pub async fn build_pending_transactions_worker(
    settings: std::sync::Arc<std::sync::Mutex<Settings>>,
    eth_keypair: std::sync::Arc<secp256k1::SecretKey>,
    redis: crate::async_redis_wrapper::AsyncRedisWrapper,
    eth_contract_abi: std::sync::Arc<String>,
    eth_contract_address: std::sync::Arc<web3::types::Address>,
) -> JoinHandle<()> {
    tokio::spawn({
        let (rpc_url, pending_transaction_poll_delay_sec, rainbow_bridge_index_js_path) = {
            let s = settings.lock().unwrap();
            (
                s.eth.rpc_url.clone(),
                s.eth.pending_transaction_poll_delay_sec,
                s.eth.rainbow_bridge_index_js_path.clone(),
            )
        };

        async move {
            pending_transactions_worker::run(
                rpc_url,
                *eth_contract_address.as_ref(),
                eth_contract_abi.as_ref().clone(),
                web3::signing::SecretKeyRef::from(eth_keypair.as_ref()),
                rainbow_bridge_index_js_path,
                redis,
                pending_transaction_poll_delay_sec as u64,
            )
            .await
        }
    })
}

pub async fn build_near_events_subscriber(
    settings: std::sync::Arc<std::sync::Mutex<Settings>>,
    eth_keypair: std::sync::Arc<secp256k1::SecretKey>,
    redis: SafeAsyncRedisWrapper,
    eth_contract_abi: std::sync::Arc<String>,
    eth_contract_address: std::sync::Arc<web3::types::Address>,
    mut stream: tokio::sync::mpsc::Receiver<String>,
    near_relay_account_id: String,
) {
    while let Some(msg) = stream.recv().await {
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
                    settings.clone(),
                    redis.clone(),
                    *eth_contract_address.as_ref(),
                    eth_keypair.clone(),
                    eth_contract_abi.clone(),
                    near_relay_account_id.clone(),
                )
                .await;

                if let Err(error) = res {
                    eprintln!(
                        "Failed to process tx with nonce {}, err: {}",
                        nonce.0, error
                    );
                }
            }
        }
    }
}
