use std::io::Write;
use near_lake_framework::{LakeConfig, LakeConfigBuilder};
use near_lake_framework::near_indexer_primitives::types::{AccountId, BlockHeight};
use near_lake_framework::near_indexer_primitives::views::{
    StateChangeValueView, StateChangeWithCauseView,
};
use std::str::FromStr;
use rocket::form::validate::Len;
//use spectre_bridge_common;
use spectre_bridge_common::Event;
use serde_json::json;

pub async fn run_watcher() {
    let config = LakeConfigBuilder::default()
        .testnet()
        .start_block_height(89175285)
        .build()
        .expect("Failed to build LakeConfig");
    //let x = spectre - bridge - protocol::near::contracts::transfer::src::;
    // instantiate the NEAR Lake Framework Stream
    let mut stream = near_lake_framework::streamer(config);

    while let Some(streamer_message) = stream.recv().await {
        //println!("\r\nBlock {} {}", streamer_message.block.header.height, streamer_message.block.author);
        for shard in streamer_message.shards {
            for outcome in shard.receipt_execution_outcomes {
                let contract_name =  AccountId::from_str("transfer.spectrebridge2.testnet").unwrap();

                if contract_name == outcome.receipt.receiver_id {
                    //println!("{} predecessor_id: {:?}, receiver_id: {:?}", streamer_message.block.header.height, outcome.receipt.predecessor_id, outcome.receipt.receiver_id);

                    for log in outcome.execution_outcome.outcome.logs {

                        println!("w0 {:?}", &log[spectre_bridge_common::EVENT_JSON_STR.len()..]);
/*
                        let r = serde_json::from_str::<serde_json::Value>(&log[transfer_event_logs::EVENT_JSON_STR.len()..]);
                        println!("w1 {:?}", r);

                        let r = serde_json::from_str::<transfer_event_logs::Event>(&log[transfer_event_logs::EVENT_JSON_STR.len()..]);
                        let expected_result_str = r#"{"standard":"nep297","version":"1.0.0","event":"spectre_bridge_transfer_event","data":{"nonce":"238","valid_till":0,"transfer":{"token":"alice","amount":"100"},"fee":{"token":"alice","amount":"100"},"recipient":[113,199,101,110,199,171,136,176,152,222,251,117,27,116,1,181,246,216,151,111]}}"#;
                        let r = serde_json::from_str::<transfer_event_logs::Event>(&expected_result_str);
                        println!("w2 {:?}", r);*/

                        //let r: serde_json::Result<transfer_event_logs::Event> = serde_json::from_value(json);

                        if let Some(json) = spectre_bridge_common::remove_prefix(log.as_str()) {
                            match get_event(json) {
                                Ok(r) => {
                                    println!("Event: {:?}", r);
                                    // TODO:
                                }
                                Err(e) => {
                                    if !matches!(e, ParceError::NotEvent){
                                        eprintln!("Log error: {:?}", e);
                                    }
                                }
                            }

                            //let r: serde_json::Result<transfer_event_logs::Event> = serde_json::from_value(json);

                            //println!("wwr {:?}", r);

                            /* println!("{}", &log.as_str()[EVENT_JSON.len()..]);

                             let r= serde_json::from_str::<serde_json::Value>(&log.as_str()[EVENT_JSON.len()..]);
                             println!("w {:?}", r);

                             let r = serde_json::from_str::<transfer_event_logs::EventMessage>(&log.as_str()[EVENT_JSON.len()..]);
                             println!("ww {:?}", r);

                             //let r: serde_json::Result<transfer_event_logs::Event> = serde_json::from_str(&log.as_str()[EVENT_JSON.len()..]);
                             let r = serde_json::from_str::<transfer_event_logs::Event>(r#"{"standard":"nep297","version":"1.0.0","event":"spectre_bridge_transfer_event","data":{"nonce":"1","valid_till":1652038871250000000,"transfer":{"token":"token.spectrebridge2.testnet","amount":"50"},"fee":{"token":"token.spectrebridge2.testnet","amount":"50"},"recipient":[0,0,84,116,232,144,148,196,77,169,139,149,78,237,234,196,149,39,29,15]}}"#);
                             println!("wwr {:?}", r);

                             let r = parce_event_json(&log.as_str()[EVENT_JSON.len()..]);
                             match r {
                                 Ok(r) => {
                                     //println!("Event: {}, data: {}", r.event, r.data);
                                     // TODO:
                                 }
                                 Err(e) => {
                                     if !matches!(e, ParceError::NotEvent){
                                         eprintln!("Log error: {:?}", e);
                                     }
                                 }
                             }*/
                        }
                        //println!("log: {}", log);
                    }
                }

            }
        }
    }
}

pub enum Error {

}

pub struct JsonError(pub serde_json::Error);

#[derive(Debug)]
pub enum ParceError {
    Json(serde_json::Error),
    WrongVersion(String),
    NotEvent,
    Other
}

pub fn fix_json(mut json: serde_json::Value) -> serde_json::Value {
    if let Some(data) = json.get_mut("data") {
        if let Some (arr) = data.as_array_mut() {
            if let Some(item) = arr.get_mut(0) {
                *data = item.take();
            }
        }
    }

    json
}

pub fn get_event(mut json: serde_json::Value) -> Result<spectre_bridge_common::Event, ParceError> {
    let mut json = fix_json(json);

    let r = serde_json::from_value::<spectre_bridge_common::EventMessage>(json.clone());  // TODO: try to remove "clone"
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

fn parce_event_message(json: &str) -> serde_json::Result<spectre_bridge_common::EventMessage> {
    let r: serde_json::Result<spectre_bridge_common::EventMessage> = serde_json::from_str(json);
    r
}

fn parce_event_json(json: &str) -> Result<spectre_bridge_common::EventMessage, ParceError> {
    let r: serde_json::Result<spectre_bridge_common::EventMessage> = serde_json::from_str(json);
    let r = r.map_err(|e| ParceError::Json(e))?;

    if r.standard != spectre_bridge_common::STANDARD {
        return Err(ParceError::NotEvent);
    }

    if r.version != spectre_bridge_common::VERSION {
        return Err(ParceError::WrongVersion(r.version));
    }

    Ok(r)
}
/*
#[cfg(test)]
pub mod tests {
    use std::str::FromStr;

    #[test]
    fn parce() {

    }
}*/