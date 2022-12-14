use crate::{pending_transactions_worker, Settings};
use rocket::Rocket;
use tokio::task::JoinHandle;

mod rocket_endpoints;

pub async fn request_interval(seconds: u64) -> tokio::time::Interval {
    tokio::time::interval_at(
        tokio::time::Instant::now() + tokio::time::Duration::from_secs(seconds),
        tokio::time::Duration::from_secs(seconds),
    )
}

pub fn build_rocket(
    conf: rocket::Config,
    settings: std::sync::Arc<std::sync::Mutex<Settings>>,
    storage: std::sync::Arc<std::sync::Mutex<crate::last_block::Storage>>,
    async_redis: std::sync::Arc<std::sync::Mutex<crate::async_redis_wrapper::AsyncRedisWrapper>>,
) -> Rocket<rocket::Build> {
    rocket::build()
        .configure(conf)
        .mount(
            "/v1",
            rocket::routes![
                rocket_endpoints::health,
                rocket_endpoints::transactions,
                rocket_endpoints::set_threshold,
                rocket_endpoints::set_allowed_tokens,
                rocket_endpoints::profit,
                rocket_endpoints::set_mapped_tokens,
                rocket_endpoints::get_mapped_tokens,
                rocket_endpoints::insert_mapped_tokens,
                rocket_endpoints::remove_mapped_tokens,
            ],
        )
        .manage(settings)
        .manage(storage)
        .manage(async_redis)
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
    redis: std::sync::Arc<std::sync::Mutex<crate::async_redis_wrapper::AsyncRedisWrapper>>,
    eth_contract_abi: std::sync::Arc<String>,
    eth_contract_address: std::sync::Arc<web3::types::Address>,
    mut stream: tokio::sync::mpsc::Receiver<String>,
) -> impl futures_util::Future<Output = ()> {
    async move {
        while let Some(msg) = stream.recv().await {
            if let Ok(event) = serde_json::from_str::<spectre_bridge_common::Event>(msg.as_str()) {
                println!("Process event {:?}", event);

                if let spectre_bridge_common::Event::SpectreBridgeInitTransferEvent {
                    nonce,
                    chain_id,
                    valid_till,
                    transfer,
                    fee,
                    recipient,
                } = event
                {
                    crate::event_processor::process_transfer_event(
                        nonce,
                        chain_id,
                        valid_till,
                        transfer,
                        fee,
                        recipient,
                        settings.clone(),
                        redis.clone(),
                        *eth_contract_address.as_ref(),
                        eth_keypair.clone(),
                        eth_contract_abi.clone(),
                    );
                }
            }
        }
    }
}
