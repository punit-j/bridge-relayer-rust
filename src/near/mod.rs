pub mod near_event;

use near_lake_framework::LakeConfig;
use near_lake_framework::near_indexer_primitives::types::{AccountId, BlockHeight};
use near_lake_framework::near_indexer_primitives::views::{
    StateChangeValueView, StateChangeWithCauseView,
};
use std::str::FromStr;

pub async fn run_watcher() {
    let config = LakeConfig {
        s3_endpoint: None,
        s3_bucket_name: "near-lake-data-testnet".to_string(), // AWS S3 bucket name
        s3_region_name: "eu-central-1".to_string(), // AWS S3 bucket region
        start_block_height: 87831762-10, // the latest block height we've got from explorer.near.org for testnet
    };
    //let x = spectre - bridge - protocol::near::contracts::transfer::src::;
    // instantiate the NEAR Lake Framework Stream
    let mut stream = near_lake_framework::streamer(config);

    while let Some(streamer_message) = stream.recv().await {
        //println!("\r\nBlock {} {}", streamer_message.block.header.height, streamer_message.block.author);
        for shard in streamer_message.shards {
            for outcome in shard.receipt_execution_outcomes {

                let watching_list = &[AccountId::from_str("br_misha.testnet").unwrap(), AccountId::from_str("weth_beta.nearlend.testnet").unwrap()];

                if watching_list.contains(&outcome.receipt.receiver_id) {
                    println!("predecessor_id: {:?}, receiver_id: {:?}", outcome.receipt.predecessor_id, outcome.receipt.receiver_id);

                    for log in outcome.execution_outcome.outcome.logs {
                        let EVENT_JSON: &'static str = "EVENT_JSON:";
                        if log.starts_with(EVENT_JSON) {
                            let r = parce_event_json(&log.as_str()[EVENT_JSON.len()..]);

                            println!("Log: {:?}", r);
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

#[derive(Debug)]
pub enum ParceError {
    Json(serde_json::Error),
    WrongVersion(String),
    WrongStandart(String),
    Other,
}

fn parce_event_message(json: &str) -> serde_json::Result<near_event::EventMessage> {
    let r: serde_json::Result<near_event::EventMessage> = serde_json::from_str(json);
    r
}

fn parce_event_json(json: &str) -> Result<near_event::EventMessage, ParceError> {
    let r: serde_json::Result<near_event::EventMessage> = serde_json::from_str(json);
    let r = r.map_err(|e| ParceError::Json(e))?;

    if r.version != near_event::VERSION {
        return Err(ParceError::WrongVersion(r.version));
    }
    if r.standard != near_event::STANDARD {
        return Err(ParceError::WrongStandart(r.standard));
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