use crate::prometheus_metrics::{INIT_TRANSFERS_COUNT, NEAR_LAST_PROCESSED_BLOCK_HEIGHT};
use crate::{async_redis_wrapper::AsyncRedisWrapper, config::NearNetwork};
use fast_bridge_common::Event;
use near_lake_framework::{near_indexer_primitives::types::AccountId, LakeConfigBuilder};
use std::thread::sleep;
use std::time;
use tracing::log::warn;

pub const OPTION_START_BLOCK: &str = "START_BLOCK";

macro_rules! info {
    ($($arg:tt)+) => { tracing::info!(target: crate::logs::NEAR_EVENTS_TRACER_TARGET, $($arg)+) }
}

macro_rules! error {
    ($($arg:tt)+) => { tracing::error!(target: crate::logs::NEAR_EVENTS_TRACER_TARGET, $($arg)+) }
}

macro_rules! trace {
    ($($arg:tt)+) => { tracing::trace!(target: crate::logs::NEAR_EVENTS_TRACER_TARGET, $($arg)+) }
}

#[allow(clippy::await_holding_lock)]
// extract all events produced by contract_name on NEAR
// since start_block and save it to redis.
pub async fn run_worker(
    contract_name: AccountId,
    mut redis: AsyncRedisWrapper,
    start_block: u64,
    near_network: NearNetwork,
) {
    let mut lake_config = LakeConfigBuilder::default().start_block_height(start_block);

    lake_config = match near_network {
        NearNetwork::Mainnet => lake_config.mainnet(),
        NearNetwork::Testnet => lake_config.testnet(),
    };

    info!("NEAR lake starts from block {}", start_block);

    let (_, mut stream) =
        near_lake_framework::streamer(lake_config.build().expect("Failed to build LakeConfig"));

    while let Some(streamer_message) = stream.recv().await {
        trace!(
            "Process near block {}",
            streamer_message.block.header.height
        );
        NEAR_LAST_PROCESSED_BLOCK_HEIGHT.set(streamer_message.block.header.height);
        for shard in streamer_message.shards {
            for outcome in shard.receipt_execution_outcomes {
                if contract_name == outcome.receipt.receiver_id {
                    info!("Process receipt {}", outcome.receipt.receipt_id);

                    for log in outcome.execution_outcome.outcome.logs {
                        if let Some(json) = fast_bridge_common::remove_prefix(log.as_str()) {
                            match get_event(json) {
                                Ok(r) => {
                                    info!(
                                        "New event: {}",
                                        serde_json::to_string(&r).unwrap_or(format!("{:?}", r))
                                    );

                                    if let Event::FastBridgeInitTransferEvent { .. } = r {
                                        INIT_TRANSFERS_COUNT.inc();
                                    }

                                    #[cfg(feature = "integration_tests")]
                                    redis.event_pub(r.clone()).await;
                                    loop {
                                        if let Err(error) = redis.store_new_event(&r.clone()).await
                                        {
                                            warn!("Error on storing new event into redis: {:?}. Try again after 15s.", error);
                                            sleep(time::Duration::from_secs(15));
                                            continue;
                                        }
                                        break;
                                    }
                                }
                                Err(e) => {
                                    if !matches!(e, ParceError::NotEvent) {
                                        error!("Log error: {:?}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // store block number to redis
        let start_block = streamer_message.block.header.height as u64 + 1;
        redis
            .option_set(OPTION_START_BLOCK, start_block)
            .await
            .unwrap();
    }
}

#[derive(Debug)]
pub enum ParceError {
    Json(serde_json::Error),
    WrongVersion(String),
    NotEvent,
}

/// In case if the "data" is array (with 1 item) it converts to object
pub fn fix_json(mut json: serde_json::Value) -> serde_json::Value {
    if let Some(data) = json.get_mut("data") {
        if let Some(arr) = data.as_array_mut() {
            if arr.len() == 1 {
                if let Some(item) = arr.get_mut(0) {
                    *data = item.take();
                }
            }
        }
    }

    json
}

/// Gets an event from json and checks standard+version
pub fn get_event(json: serde_json::Value) -> Result<fast_bridge_common::Event, ParceError> {
    let json = fix_json(json);

    let r = serde_json::from_value::<fast_bridge_common::EventMessage>(json.clone());
    let r = r.map_err(ParceError::Json)?;

    if r.standard != fast_bridge_common::STANDARD {
        return Err(ParceError::NotEvent);
    }

    if r.version != fast_bridge_common::VERSION {
        return Err(ParceError::WrongVersion(r.version));
    }

    let r = serde_json::from_value::<fast_bridge_common::Event>(json);
    let r = r.map_err(ParceError::Json)?;

    Ok(r)
}

#[cfg(test)]
pub mod tests {
    use crate::config::NearNetwork;
    use crate::near_events_tracker::{fix_json, get_event, run_worker};
    use assert_json_diff::assert_json_eq;
    use fast_bridge_common;
    use near_sdk::json_types::U128;
    use near_sdk::AccountId;

    use crate::async_redis_wrapper::{subscribe, AsyncRedisWrapper, EVENTS};
    use crate::logs::init_logger;
    use crate::test_utils::{get_settings, NEAR_CONTRACT_ADDRESS};
    use serde_json::json;
    use tokio::time::timeout;

    #[test]
    fn fix_json_test() {
        let json_valid = json!({"data": 1});

        let json = json!({"data": 1});
        assert_json_eq!(fix_json(json), json_valid);

        let json = json!({"data": [1]});
        assert_json_eq!(fix_json(json), json_valid);

        let json = json!({"data": [1, 2]});
        assert_json_eq!(fix_json(json.clone()), json);
    }

    #[test]
    fn get_event_test() {
        let json_str = r#"EVENT_JSON:{"standard":"nep297","version":"1.0.0","event":"fast_bridge_deposit_event","data":{"amount":"179","sender_id":"alice","token":"token"}}"#;
        let json = fast_bridge_common::remove_prefix(json_str).unwrap();
        let event = get_event(json).unwrap();

        assert_eq!(
            event,
            fast_bridge_common::Event::FastBridgeDepositEvent {
                sender_id: AccountId::new_unchecked("alice".to_string()),
                token: AccountId::new_unchecked("token".to_string()),
                amount: U128(179)
            }
        )
    }

    #[tokio::test]
    // Should be created AWS account and key saved to ~/.aws/credentials
    async fn smoke_run_worker_test() {
        init_logger();

        let settings = get_settings();
        let contract_address =
            crate::near_events_tracker::AccountId::try_from(NEAR_CONTRACT_ADDRESS.to_string())
                .unwrap();
        let init_block = 113576799;
        let settings = std::sync::Arc::new(tokio::sync::Mutex::new(settings));

        let redis = AsyncRedisWrapper::connect(&settings.lock().await.redis).await;

        let worker = run_worker(
            contract_address,
            redis.clone(),
            init_block,
            NearNetwork::Testnet,
        );

        let mut stream = subscribe::<String>(EVENTS.to_string(), redis.clone()).unwrap();

        let timeout_duration = std::time::Duration::from_secs(60);
        let _result = timeout(timeout_duration, worker).await;

        let recv_event =
            serde_json::from_str::<fast_bridge_common::Event>(&stream.recv().await.unwrap())
                .unwrap();
        println!("recv event: {:?}", recv_event);
    }
}
