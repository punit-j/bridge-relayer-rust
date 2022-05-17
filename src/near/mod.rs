//pub mod near_event;

use std::io::Write;
use near_lake_framework::{LakeConfig, LakeConfigBuilder};
use near_lake_framework::near_indexer_primitives::types::{AccountId, BlockHeight};
use near_lake_framework::near_indexer_primitives::views::{
    StateChangeValueView, StateChangeWithCauseView,
};
use std::str::FromStr;
use transfer_event_custon_logs as cust_events;

use near_sdk::{log, serde_json, serde::Serialize, serde::Deserialize};

//use my_librrr;

pub async fn run_watcher() {
    let config = LakeConfigBuilder::default()
        .testnet()
        .start_block_height(90027479)
        .build()
        .expect("Failed to build LakeConfig");
    //let x = spectre - bridge - protocol::near::contracts::transfer::src::;
    // instantiate the NEAR Lake Framework Stream
    let mut stream = near_lake_framework::streamer(config);

    while let Some(streamer_message) = stream.recv().await {
        let block_str = serde_json::to_string(&streamer_message).unwrap();
       /* if streamer_message.block.header.height != 90027479 {
            continue;
        }

        println!("ms {}", sss);*/
       // continue;
        if block_str.contains("nep297") {
            println!("ms {}", block_str);
        }

        //println!("\r\nBlock {} {}", streamer_message.block.header.height, streamer_message.block.author);
        for shard in streamer_message.shards {
            /*let sss = serde_json::to_string(&shard).unwrap();
            if !sss.contains("AVsQPZGamsnkeaZ3bkFaU8iaQYSSJB5zG8Kf4jcf7i9A") {
                continue;
            }
            println!("ms {}", sss);

            if let Some(chunk) = shard.chunk {

            }*/

            for outcome in shard.receipt_execution_outcomes {
                let contract_name =  AccountId::from_str("dweth_beta.nearlend.testnet").unwrap();

                if /*contract_name == outcome.receipt.receiver_id*/1==1 {
                    //println!("{} predecessor_id: {:?}, receiver_id: {:?}", streamer_message.block.header.height, outcome.receipt.predecessor_id, outcome.receipt.receiver_id);

                    for log in outcome.execution_outcome.outcome.logs {
                           let EVENT_JSON: &'static str = "EVENT_JSON:";
                        if log.starts_with(EVENT_JSON) {
                            let r = parce_event_json(&log.as_str()[EVENT_JSON.len()..]);
                            if block_str.contains("nep297") {
                                println!("l {}", &log.as_str()[EVENT_JSON.len()..]);
                            }

                            match r {
                                Ok(r) => {
                                    println!("Event: {}, data: {}", r.event, r.data[0]);
                                }
                                Err(e) => {
                                    if !matches!(e, ParceError::NotEvent){
                                        eprintln!("Log error: {:?}", e);
                                    }
                                }
                            }
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

fn parce_event_message(json: &str) -> serde_json::Result<cust_events::EventMessage> {
    let r: serde_json::Result<cust_events::EventMessage> = serde_json::from_str(json);
    r
}
/*
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct EventMessage {
    pub standard: String,
    pub version: String,
    pub event: serde_json::Value,
    pub data: serde_json::Value
}*/

fn parce_event_json(json: &str) -> Result<cust_events::EventMessage, ParceError> {
    let r: serde_json::Result<cust_events::EventMessage> = serde_json::from_str(json);
    let r = r.map_err(|e| ParceError::Json(e))?;

    if r.standard != cust_events::STANDARD {
        return Err(ParceError::NotEvent);
    }

    if r.version != cust_events::VERSION {
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