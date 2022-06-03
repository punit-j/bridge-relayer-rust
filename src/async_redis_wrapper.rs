use futures_util::StreamExt;
use redis::{AsyncCommands, RedisResult};
use std::sync;

#[derive(Clone)]
pub struct AsyncRedisWrapper {
    pub client: redis::Client,
    pub connection: redis::aio::MultiplexedConnection,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransactionData {
    pub block: u64,
    pub proof: spectre_bridge_common::Proof,
    pub nonce: u128,
}

pub const OPTIONS: &str = "options";

// Set of pairs <TX_HASH, TX_DATA>
pub const TRANSACTIONS: &str = "transactions";

// Transaction queue
pub const TRANSACTION_HASHES: &str = "transaction_hashes";

// Transaction queue
pub const EVENTS: &str = "events";

// TODO: review. Moved from the redis_wrapper
const REDIS_TRANSACTION_HASH: &str = "myhash";
const REDIS_PROFIT_HASH: &str = "myprofit";

impl AsyncRedisWrapper {
    pub async fn connect(settings: crate::config::RedisSettings) -> Self {
        let client = redis::Client::open(settings.url.clone())
            .expect("REDIS: Failed to establish connection");
        let connection = client
            .get_multiplexed_tokio_connection()
            .await
            .expect("REDIS: Failed to get connection");
        AsyncRedisWrapper { client, connection }
    }

    pub async fn option_set<T: redis::ToRedisArgs + Send + Sync>(&mut self, name: &str, value: T) -> redis::RedisResult<()> {
        self.connection.hset(OPTIONS, name, value).await?;
        Ok(())
    }

    pub async fn option_get<T: redis::ToRedisArgs + Send + Sync + redis::FromRedisValue>(&mut self, name: &str) -> redis::RedisResult<Option<T>> {
        let val: Option<T> = self.connection.hget(OPTIONS, name).await?;
        Ok(val)
    }

    pub async fn event_pub(&mut self, event: spectre_bridge_common::Event) {
        let _: () = self.connection.publish(EVENTS, serde_json::to_string(&event).unwrap()).await.unwrap();
    }

    pub async fn hset(
        &mut self,
        tx_hash: String,
        tx_data: TransactionData,
    ) -> redis::RedisResult<()> {
        self.connection
            .hset(
                TRANSACTIONS,
                tx_hash,
                serde_json::to_string(&tx_data).expect(""),
            )
            .await?;
        Ok(())
    }

    pub async fn rpush(&mut self, tx_hash: String) -> redis::RedisResult<()> {
        Ok(self.connection.rpush(TRANSACTION_HASHES, tx_hash).await?)
    }

    pub async fn hget(&mut self, tx_hash: String) -> redis::RedisResult<TransactionData> {
        let serialized_data: String = self.connection.hget(TRANSACTIONS, tx_hash).await?;
        Ok(serde_json::from_str(&serialized_data).expect(""))
    }

    pub async fn hdel(&mut self, tx_hash: String) -> redis::RedisResult<()> {
        Ok(self.connection.hdel(TRANSACTIONS, tx_hash).await?)
    }

    pub async fn hkeys(&mut self) -> redis::RedisResult<Vec<String>> {
        Ok(self.connection.hkeys(TRANSACTIONS).await?)
    }

    pub async fn lpop(&mut self) -> redis::RedisResult<Option<String>> {
        let tx_hashes: Vec<String> = self
            .connection
            .lpop(TRANSACTION_HASHES, std::num::NonZeroUsize::new(1))
            .await?;
        match tx_hashes.is_empty() {
            true => Ok(None),
            false => Ok(Some(tx_hashes[0].clone())),
        }
    }

    // TODO: review. Moved from the redis_wrapper
    pub async fn get_all(&mut self) -> Vec<String> {
        let result: Vec<String> = self.connection.hvals(REDIS_TRANSACTION_HASH).await.unwrap();
        result
    }

    // TODO: review. Moved from the redis_wrapper
    pub async fn _increase_profit(&mut self, add_to: u64) -> RedisResult<()> {
        let profit: i32 = self
            .connection
            .hget(REDIS_PROFIT_HASH, "profit".to_string()).await
            .ok()
            .unwrap_or(0); // In case we don't have initial value in DB

        self.connection.hset(
            REDIS_PROFIT_HASH,
            "profit".to_string(),
            add_to + profit as u64,
        ).await?;

        Ok(())
    }

    // TODO: review. Moved from the redis_wrapper
    pub async fn get_profit(&mut self) -> u64 {
        self.connection
            .hget(REDIS_PROFIT_HASH, "profit".to_string()).await
            .ok()
            .unwrap()
    }
}

pub fn subscribe<T: 'static + redis::FromRedisValue + Send>(channel: String,
                                                           redis: std::sync::Arc<std::sync::Mutex<AsyncRedisWrapper>>)
    -> RedisResult<tokio::sync::mpsc::Receiver<T>>
{
    let (sender, receiver) = tokio::sync::mpsc::channel::<T>(100);
    tokio::spawn(
        async move {
            let client = redis.lock().unwrap().client.clone();
            let mut pubsub_connection = client.get_async_connection().await.expect("REDIS: Failed to get connection").into_pubsub();
            pubsub_connection.subscribe(channel).await.expect("Failed to subscribe to the channel");
            let mut pubsub_stream = pubsub_connection.on_message();

            while let Some(s) = pubsub_stream.next().await {
                let pubsub_msg: T = s.get_payload().expect("Failed to fetch the message");
                if let Err(e) = sender.send(pubsub_msg).await {
                    break;
                }
            }
        }
    );
    Ok(receiver)
}