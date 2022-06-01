use near_lake_framework::near_indexer_primitives::types::{AccountId, BlockHeight};
use near_lake_framework::near_indexer_primitives::views::{
    StateChangeValueView, StateChangeWithCauseView,
};
use near_lake_framework::{LakeConfig, LakeConfigBuilder};
use rocket::form::validate::Len;
use std::io::Write;
use std::str::FromStr;
//use spectre_bridge_common;
use serde_json::json;
use spectre_bridge_common::Event;

pub async fn run_watcher<'a>(contract_name: &'a AccountId) {
    let config = LakeConfigBuilder::default()
        .testnet()
        .start_block_height(90587572)
        .build()
        .expect("Failed to build LakeConfig");

    let mut stream = near_lake_framework::streamer(config);

    while let Some(streamer_message) = stream.recv().await {
        for shard in streamer_message.shards {
            for outcome in shard.receipt_execution_outcomes {
                if *contract_name == outcome.receipt.receiver_id {
                    for log in outcome.execution_outcome.outcome.logs {
                        if let Some(json) = spectre_bridge_common::remove_prefix(log.as_str()) {
                            match get_event(json) {
                                Ok(r) => {
                                    println!("Event: {:?}", r);
                                    // TODO:
                                }
                                Err(e) => {
                                    if !matches!(e, ParceError::NotEvent) {
                                        eprintln!("Log error: {:?}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub struct JsonError(pub serde_json::Error);

#[derive(Debug)]
pub enum ParceError {
    Json(serde_json::Error),
    WrongVersion(String),
    NotEvent,
    Other,
}

/// In case if the "data" is array (with 1 item) it converts to object
pub fn fix_json(mut json: serde_json::Value) -> serde_json::Value {
    if let Some(data) = json.get_mut("data") {
        if let Some(arr) = data.as_array_mut() {
            if let Some(item) = arr.get_mut(0) {
                *data = item.take();
            }
        }
    }

    json
}

/// Gets an event from json and checks standard+version
pub fn get_event(mut json: serde_json::Value) -> Result<spectre_bridge_common::Event, ParceError> {
    let mut json = fix_json(json);

    let r = serde_json::from_value::<spectre_bridge_common::EventMessage>(json.clone());
    let r = r.map_err(|e| ParceError::Json(e))?;

    if r.standard != spectre_bridge_common::STANDARD {
        return Err(ParceError::NotEvent);
    }

    if r.version != spectre_bridge_common::VERSION {
        return Err(ParceError::WrongVersion(r.version));
    }

    let r = serde_json::from_value::<spectre_bridge_common::Event>(json);
    let r = r.map_err(|e| ParceError::Json(e))?;

    Ok(r)
}

#[cfg(test)]
pub mod tests {
    use crate::near::{fix_json, get_event};
    use assert_json_diff::assert_json_eq;
    use near_sdk::json_types::U128;
    use near_sdk::AccountId;
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn fix_json_test() {
        let json_valid = json!({"data": 1});

        let json = json!({"data": 1});
        assert_json_eq!(fix_json(json), json_valid.clone());

        let json = json!({"data": [1]});
        assert_json_eq!(fix_json(json), json_valid)
    }

    #[test]
    fn get_event_test() {
        let json_str = r#"EVENT_JSON:{"standard":"nep297","version":"1.0.0","event":"spectre_bridge_transfer_failed_event","data":{"nonce":"238","account":"alice"}}"#;
        let json = spectre_bridge_common::remove_prefix(json_str).unwrap();
        let event = get_event(json).unwrap();

        assert_eq!(
            event,
            spectre_bridge_common::Event::SpectreBridgeTransferFailedEvent {
                nonce: U128(238),
                account: AccountId::new_unchecked("alice".to_string()),
            }
        )
    }
}
