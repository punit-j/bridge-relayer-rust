use near_lake_framework::LakeConfig;
use near_lake_framework::near_indexer_primitives::types::{AccountId, BlockHeight};
use near_lake_framework::near_indexer_primitives::views::{
    StateChangeValueView, StateChangeWithCauseView,
};

pub async fn run() {
    let config = LakeConfig {
        s3_endpoint: None,
        s3_bucket_name: "near-lake-data-testnet".to_string(), // AWS S3 bucket name
        s3_region_name: "eu-central-1".to_string(), // AWS S3 bucket region
        start_block_height: 87831762-10, // the latest block height we've got from explorer.near.org for testnet
    };

    // instantiate the NEAR Lake Framework Stream
    let mut stream = near_lake_framework::streamer(config);

    while let Some(streamer_message) = stream.recv().await {
        //println!("\r\nBlock {} {}", streamer_message.block.header.height, streamer_message.block.author);
        for shard in streamer_message.shards {
            for outcome in shard.receipt_execution_outcomes {
                println!("predecessor_id: {:?}, receiver_id: {:?}", outcome.receipt.predecessor_id, outcome.receipt.receiver_id);
            }
        }
    }
}