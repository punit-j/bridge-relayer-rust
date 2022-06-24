use crate::{async_redis_wrapper, ethereum, ToHex};

use crate::redis::AsyncCommands;
use std::str::FromStr;

pub async fn run(
    rpc_url: url::Url,
    eth_contract_address: web3::types::Address,
    eth_contract_abi: String,
    eth_keypair: &secp256k1::SecretKey,
    mut redis: crate::async_redis_wrapper::AsyncRedisWrapper,
    delay_request_status_sec: u64,
) {
    let eth_client = ethereum::RainbowBridgeEthereumClient::new(
        rpc_url.as_str(),
        "/home/misha/trash/rr/rainbow-bridge/cli/index.js",
        eth_contract_address,
        &eth_contract_abi.as_bytes(),
        *eth_keypair,
    )
    .unwrap();

    // transaction hash and last processed time
    let mut pending_transactions: std::collections::HashMap<
        web3::types::H256,
        async_redis_wrapper::PendingTransactionData,
    > = std::collections::HashMap::new();

    loop {
        // fill the pending_transactions
        let mut iter: redis::AsyncIter<(String, String)> = redis
            .connection
            .hscan(async_redis_wrapper::PENDING_TRANSACTIONS)
            .await
            .unwrap();
        while let Some(pair) = iter.next_item().await {
            let hash =
                web3::types::H256::from_str(pair.0.as_str()).expect("Unable to parse tx hash");
            let data = serde_json::from_str::<async_redis_wrapper::PendingTransactionData>(
                pair.1.as_str(),
            )
            .unwrap();

            if !pending_transactions.contains_key(&hash) {
                pending_transactions.insert(hash, data);
                println!("New pending transaction: {:#?}", hash)
            }
        }

        // process the pending_transactions
        let mut transactions_to_remove: Vec<web3::types::H256> = Vec::new();
        for mut item in pending_transactions.iter_mut() {
            // remove and skip if transaction is already processing
            if redis
                .get_tx_data(item.0.as_bytes().to_hex::<String>())
                .await
                .is_ok()
            {
                transactions_to_remove.push(*item.0);
            } else if (item.1.timestamp + delay_request_status_sec)
                < std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            {
                match eth_client.transaction_status(*item.0).await {
                    Ok(status) => {
                        match status {
                            ethereum::transactions::TransactionStatus::Pengind => {
                                // update the timestamp
                                item.1.timestamp = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs();
                            }
                            ethereum::transactions::TransactionStatus::Failure(block_number) => {
                                println!("Transfer token transaction is failed {:?}", item.0);
                                transactions_to_remove.push(*item.0);
                            }
                            ethereum::transactions::TransactionStatus::Sucess(block_number) => {
                                let proof = eth_client.get_proof(item.0).await;
                                match proof {
                                    Ok(proof) => {
                                        let data = async_redis_wrapper::TxData {
                                            block: u64::try_from(block_number).unwrap(),
                                            proof,
                                            nonce: item.1.nonce,
                                        };
                                        let _: () = redis
                                            .store_tx(item.0.as_bytes().to_hex::<String>(), data)
                                            .await
                                            .unwrap();
                                        transactions_to_remove.push(*item.0);
                                    }
                                    Err(e) => {
                                        println!("Error on request proof: {:?}", e)
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error on request transaction status: {:?}", e)
                    }
                }
            }
        }

        for item in transactions_to_remove {
            let res: redis::RedisResult<()> = redis
                .connection
                .hdel(
                    async_redis_wrapper::PENDING_TRANSACTIONS,
                    item.as_bytes().to_hex::<String>(),
                )
                .await;
            if let Err(e) = res {
                eprintln!("Error on remove pending transaction {}", e);
            }
            pending_transactions.remove(&item);
        }

        tokio::time::sleep(core::time::Duration::from_secs(1)).await;
    }
}
