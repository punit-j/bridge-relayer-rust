use crate::async_redis_wrapper::AsyncRedisWrapper;
use crate::{async_redis_wrapper, ToHex};
use redis::AsyncCommands;
use std::sync::{Arc, Mutex};

#[allow(clippy::too_many_arguments)]
pub fn process_transfer_event(
    nonce: near_sdk::json_types::U128,
    chain_id: u32,
    valid_till: u64,
    transfer: spectre_bridge_common::TransferDataEthereum,
    fee: spectre_bridge_common::TransferDataNear,
    recipient: spectre_bridge_common::EthAddress,

    settings: std::sync::Arc<std::sync::Mutex<crate::Settings>>,
    redis: Arc<Mutex<AsyncRedisWrapper>>,
    eth_contract_address: web3::types::Address,
    eth_key: std::sync::Arc<secp256k1::SecretKey>,
    eth_contract_abi: std::sync::Arc<String>,
) {
    tokio::spawn({
        let rpc_url = settings.lock().unwrap().eth.rpc_url.clone();
        let near_addr = transfer.token_near.clone();
        let eth_contract_abi = std::sync::Arc::clone(&eth_contract_abi);
        async move {
            let mut redis = redis.lock().unwrap().clone();
            let tx_hash = crate::transfer::execute_transfer(
                eth_key.clone().as_ref(),
                spectre_bridge_common::Event::SpectreBridgeTransferEvent {
                    nonce,
                    chain_id,
                    valid_till,
                    transfer,
                    fee,
                    recipient,
                },
                eth_contract_abi.as_bytes(),
                rpc_url.as_str(),
                eth_contract_address,
                0.0,
                settings.clone(),
            )
            .await;

            match tx_hash {
                Ok(Some(hash)) => {
                    println!("New eth transaction: {:#?}", hash);

                    let d = crate::async_redis_wrapper::PendingTransactionData {
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        nonce: u128::from(nonce),
                    };

                    let res: redis::RedisResult<()> = redis
                        .connection
                        .hset(
                            async_redis_wrapper::PENDING_TRANSACTIONS,
                            hash.as_bytes().to_hex::<String>(),
                            serde_json::to_string(&d).unwrap(),
                        )
                        .await;
                    if let Err(e) = res {
                        eprintln!("Unable to store pending transaction: {}", e);
                    }
                }
                Ok(None) => {
                    println!(
                        "Transaction {} is not profitable: {}",
                        u128::from(nonce),
                        near_addr
                    );
                }
                Err(error) => {
                    eprint!("Failed to execute transferTokens: {}", error)
                }
            }
        }
    });
}
