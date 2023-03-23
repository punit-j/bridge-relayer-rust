use futures_util::StreamExt;
use redis::{AsyncCommands, RedisResult};
use web3::types::U256;

#[derive(Clone)]
pub struct AsyncRedisWrapper {
    pub client: redis::Client,
    pub connection: redis::aio::MultiplexedConnection,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TxData {
    pub block: u64,
    pub proof: fast_bridge_common::Proof,
    pub nonce: u128,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PendingTransactionData {
    pub timestamp: u64,
    pub nonce: u128,
}

pub const OPTIONS: &str = "options";
pub const OPTION_ETH_TRANSACTION_COUNT: &str = "ETH_TRANSACTION_COUNT";

// Set of pairs <TX_HASH, TX_DATA>
pub const TRANSACTIONS: &str = "transactions";

// Transaction queue
pub const EVENTS: &str = "events";

pub const PENDING_TRANSACTIONS: &str = "pending_transactions";

impl AsyncRedisWrapper {
    pub async fn connect(redis_settings: &crate::config::RedisSettings) -> Self {
        let client = redis::Client::open(redis_settings.url.clone())
            .expect("REDIS: Failed to establish connection");
        let connection = client
            .get_multiplexed_tokio_connection()
            .await
            .expect("REDIS: Failed to get connection");
        tracing::info!("Connected to the redis: {:?}", client.get_connection_info());
        AsyncRedisWrapper { client, connection }
    }

    pub async fn option_set<T: redis::ToRedisArgs + Send + Sync>(
        &mut self,
        name: &str,
        value: T,
    ) -> redis::RedisResult<()> {
        Ok(self.connection.hset(OPTIONS, name, value).await?)
    }

    pub async fn option_get<T: redis::ToRedisArgs + Send + Sync + redis::FromRedisValue>(
        &mut self,
        name: &str,
    ) -> redis::RedisResult<Option<T>> {
        Ok(self.connection.hget(OPTIONS, name).await?)
    }

    pub async fn set_transaction_count(
        &mut self,
        tx_count: web3::types::U256,
    ) -> redis::RedisResult<()> {
        self.option_set(OPTION_ETH_TRANSACTION_COUNT, tx_count.to_string())
            .await
    }

    pub async fn get_transaction_count(&mut self) -> redis::RedisResult<Option<U256>> {
        let tx_count_str: Option<String> = self.option_get(OPTION_ETH_TRANSACTION_COUNT).await?;
        match tx_count_str {
            Some(tx_count) => Ok(Some(
                U256::from_dec_str(&tx_count)
                    .expect("REDIS: Failed to deserialize the transaction count"),
            )),
            None => Ok(None),
        }
    }

    #[allow(clippy::let_unit_value)]
    pub async fn event_pub(&mut self, event: fast_bridge_common::Event) {
        let _: () = self
            .connection
            .publish(EVENTS, serde_json::to_string(&event).unwrap())
            .await
            .unwrap();
    }

    pub async fn store_tx(&mut self, tx_hash: String, tx_data: TxData) -> redis::RedisResult<()> {
        match self
            .connection
            .hset_nx(
                TRANSACTIONS,
                &tx_hash,
                &serde_json::to_string(&tx_data)
                    .expect("REDIS: Failed to serialize transaction data"),
            )
            .await
        {
            Ok(redis::Value::Int(1)) => Ok(()),
            storing_status => Err(storing_status.unwrap_err()),
        }
    }

    pub async fn unstore_tx(&mut self, tx_hash: String) -> redis::RedisResult<()> {
        match self.connection.hdel(TRANSACTIONS, &tx_hash).await {
            Ok(redis::Value::Int(1)) => Ok(()),
            unstoring_status => Err(unstoring_status.unwrap_err()),
        }
    }

    pub async fn get_tx_data(&mut self, tx_hash: String) -> redis::RedisResult<TxData> {
        match self.connection.hget(TRANSACTIONS, &tx_hash).await {
            Ok(value) => {
                let serialized_tx_data: String = redis::from_redis_value(&value)?;
                Ok(serde_json::from_str(&serialized_tx_data)
                    .expect("REDIS: Failed to deserialize transaction data"))
            }
            Err(error) => Err(error),
        }
    }

    pub async fn get_tx_hashes(&mut self) -> redis::RedisResult<Vec<String>> {
        self.connection.hkeys(TRANSACTIONS).await
    }
}

pub fn subscribe<T: 'static + redis::FromRedisValue + Send>(
    channel: String,
    redis: AsyncRedisWrapper,
) -> RedisResult<tokio::sync::mpsc::Receiver<T>> {
    let (sender, receiver) = tokio::sync::mpsc::channel::<T>(100);
    tokio::spawn(async move {
        let mut pubsub_connection = redis
            .client
            .get_async_connection()
            .await
            .expect("REDIS: Failed to get connection")
            .into_pubsub();

        let err_msg = "REDIS: Failed to subscribe to the channel";
        pubsub_connection.subscribe(channel).await.expect(err_msg);
        let mut pubsub_stream = pubsub_connection.on_message();

        while let Some(s) = pubsub_stream.next().await {
            let pubsub_msg: T = s.get_payload().expect("Failed to fetch the message");
            if let Err(_e) = sender.send(pubsub_msg).await {
                break;
            }
        }
    });
    Ok(receiver)
}

#[cfg(test)]
pub mod tests {
    use crate::async_redis_wrapper::{subscribe, AsyncRedisWrapper, TxData, EVENTS, TRANSACTIONS};
    use crate::test_utils::{get_settings, remove_all};
    use eth_client::test_utils::{get_eth_token, get_recipient};
    use fast_bridge_common::{EthAddress, TransferDataEthereum, TransferDataNear, TransferMessage};
    use near_client::test_utils::get_near_token;
    use near_sdk::json_types::U128;
    use tokio::time::Duration;

    // run `redis-server` in the terminal
    #[tokio::test]
    async fn smoke_connect_test() {
        let settings = std::sync::Arc::new(tokio::sync::Mutex::new(get_settings()));

        let _redis = AsyncRedisWrapper::connect(&settings.lock().await.redis).await;
    }

    #[tokio::test]
    async fn smoke_option_set_test() {
        let settings = std::sync::Arc::new(tokio::sync::Mutex::new(get_settings()));

        let mut redis = AsyncRedisWrapper::connect(&settings.lock().await.redis).await;
        redis.option_set("START_BLOCK", 10).await.unwrap();
    }

    #[tokio::test]
    async fn smoke_option_get_test() {
        let settings = std::sync::Arc::new(tokio::sync::Mutex::new(get_settings()));

        let mut redis = AsyncRedisWrapper::connect(&settings.lock().await.redis).await;

        redis.option_set("START_BLOCK", 10u64).await.unwrap();
        let start_block: u64 = redis.option_get("START_BLOCK").await.unwrap().unwrap();

        assert_eq!(10u64, start_block);
    }

    #[tokio::test]
    async fn smoke_tx_test() {
        let settings = std::sync::Arc::new(tokio::sync::Mutex::new(get_settings()));

        let mut redis = AsyncRedisWrapper::connect(&settings.lock().await.redis).await;

        remove_all(redis.clone(), TRANSACTIONS).await;

        let tx_hash = "test_tx_hash".to_string();
        let tx_data = TxData {
            block: 126u64,
            proof: fast_bridge_common::Proof::default(),
            nonce: 15u128,
        };

        redis
            .store_tx(tx_hash.clone(), tx_data.clone())
            .await
            .unwrap();

        let extracted_tx_data = redis.get_tx_data(tx_hash.clone()).await.unwrap();
        assert_eq!(extracted_tx_data.nonce, tx_data.nonce);

        let tx_list = redis.get_tx_hashes().await.unwrap();
        assert_eq!(tx_list.len(), 1);
        assert_eq!(tx_list[0], tx_hash.clone());

        redis.unstore_tx(tx_hash.clone()).await.unwrap();
        assert!(redis.get_tx_data(tx_hash).await.is_err());
    }

    #[tokio::test]
    async fn smoke_subscribe_test() {
        let settings = std::sync::Arc::new(tokio::sync::Mutex::new(get_settings()));

        let mut redis = AsyncRedisWrapper::connect(&settings.lock().await.redis).await;

        let mut stream = subscribe::<String>(EVENTS.to_string(), redis.clone()).unwrap();

        tokio::time::sleep(Duration::from_secs(1)).await;

        redis
            .event_pub(fast_bridge_common::Event::FastBridgeUnlockEvent {
                nonce: U128::from(16u128),
                recipient_id: "test.account".parse().unwrap(),
                transfer_message: TransferMessage {
                    valid_till: 0,
                    transfer: TransferDataEthereum {
                        token_near: get_near_token(),
                        token_eth: EthAddress(get_eth_token().into()),
                        amount: U128(1),
                    },
                    fee: TransferDataNear {
                        token: get_near_token(),
                        amount: U128(1),
                    },
                    recipient: EthAddress(get_recipient().into()),
                    valid_till_block_height: None,
                    aurora_sender: None,
                },
            })
            .await;

        let recv_event =
            serde_json::from_str::<fast_bridge_common::Event>(&stream.recv().await.unwrap())
                .unwrap();
        println!("recv event: {:?}", recv_event);
    }
}
