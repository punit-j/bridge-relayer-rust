use crate::async_redis_wrapper;
use crate::async_redis_wrapper::AsyncRedisWrapper;
use crate::config::Settings;
use near_sdk::AccountId;
use redis::AsyncCommands;
use std::sync::{Arc, Mutex};
use uint::rustc_hex::ToHex;

#[allow(clippy::too_many_arguments)]
pub fn process_transfer_event(
    nonce: near_sdk::json_types::U128,
    sender_id: AccountId,
    transfer_message: spectre_bridge_common::TransferMessage,
    settings: std::sync::Arc<std::sync::Mutex<Settings>>,
    redis: Arc<Mutex<AsyncRedisWrapper>>,
    eth_erc20_fast_bridge_proxy_contract_address: web3::types::Address,
    relay_eth_key: std::sync::Arc<secp256k1::SecretKey>,
    eth_erc20_fast_bridge_contract_abi: std::sync::Arc<String>,
    near_relay_account_id: String,
) {
    tokio::spawn({
        let rpc_url = settings.lock().unwrap().eth.rpc_url.clone();
        let profit_thershold = settings.lock().unwrap().profit_thershold;
        let eth_contract_abi = std::sync::Arc::clone(&eth_erc20_fast_bridge_contract_abi);
        async move {
            let mut redis = redis.lock().unwrap().clone();
            println!("Execute transfer on eth with nonce {:?}", nonce);
            let tx_hash = crate::transfer::execute_transfer(
                relay_eth_key.clone().as_ref(),
                spectre_bridge_common::Event::SpectreBridgeInitTransferEvent {
                    nonce,
                    sender_id,
                    transfer_message,
                },
                eth_contract_abi.as_bytes(),
                rpc_url.as_str(),
                eth_erc20_fast_bridge_proxy_contract_address,
                profit_thershold,
                settings.clone(),
                near_relay_account_id,
            )
            .await;

            match tx_hash {
                Ok(hash) => {
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
                    if let Err(error) = res {
                        eprintln!(
                            "{}",
                            crate::errors::CustomError::FailedStorePendingTx(error)
                        );
                    }
                }
                Err(error) => {
                    eprintln!(
                        "Failed to process tx with nonce {}, err: {}",
                        nonce.0, error
                    )
                }
            }
        }
    });
}

#[cfg(test)]
pub mod tests {
    use crate::async_redis_wrapper::{AsyncRedisWrapper, PENDING_TRANSACTIONS};
    use crate::event_processor::process_transfer_event;
    use crate::test_utils::get_settings;
    use eth_client::test_utils::{
        get_eth_erc20_fast_bridge_contract_abi, get_eth_erc20_fast_bridge_proxy_contract_address,
        get_eth_rpc_url, get_eth_token, get_recipient, get_relay_eth_key,
    };
    use near_client::test_utils::{get_near_signer, get_near_token};
    use near_sdk::json_types::U128;
    use rand::Rng;
    use redis::AsyncCommands;
    use spectre_bridge_common::{
        EthAddress, TransferDataEthereum, TransferDataNear, TransferMessage,
    };
    use std::time::Duration;

    #[tokio::test]
    async fn smoke_process_transfer_event_test() {
        let nonce = U128::from(rand::thread_rng().gen_range(0..1000000000));
        let valid_till = 0;
        let transfer = TransferDataEthereum {
            token_near: get_near_token(),
            token_eth: EthAddress::from(get_eth_token()),
            amount: U128::from(1),
        };
        let fee = TransferDataNear {
            token: get_near_token(),
            amount: U128::from(10),
        };
        let recipient = EthAddress::from(get_recipient());

        let mut settings = get_settings();
        settings.eth.rpc_url = get_eth_rpc_url();
        let settings = std::sync::Arc::new(std::sync::Mutex::new(settings));
        let redis = AsyncRedisWrapper::connect(settings.clone()).await;

        let arc_redis = std::sync::Arc::new(std::sync::Mutex::new(redis));

        let relay_eth_key = std::sync::Arc::new(get_relay_eth_key());
        let eth_erc20_fast_bridge_contract_abi =
            std::sync::Arc::new(get_eth_erc20_fast_bridge_contract_abi().await);

        let near_account = get_near_signer().account_id.to_string();

        let pending_transactions: Vec<String> = arc_redis
            .lock()
            .unwrap()
            .connection
            .hkeys(PENDING_TRANSACTIONS)
            .await
            .unwrap();

        process_transfer_event(
            nonce,
            near_account.parse().unwrap(),
            TransferMessage {
                valid_till: valid_till,
                transfer,
                fee,
                recipient,
                valid_till_block_height: None,
            },
            settings.clone(),
            arc_redis.clone(),
            get_eth_erc20_fast_bridge_proxy_contract_address(),
            relay_eth_key.clone(),
            eth_erc20_fast_bridge_contract_abi.clone(),
            near_account,
        );

        tokio::time::sleep(Duration::from_secs(60)).await;

        let new_pending_transactions: Vec<String> = arc_redis
            .lock()
            .unwrap()
            .connection
            .hkeys(PENDING_TRANSACTIONS)
            .await
            .unwrap();
        assert_eq!(
            pending_transactions.len() + 1,
            new_pending_transactions.len()
        );
    }
}
