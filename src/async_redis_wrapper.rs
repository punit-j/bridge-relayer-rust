use futures_util::StreamExt;
use redis::{AsyncCommands, RedisResult};

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

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PendingTransactionData {
    pub timestamp: u64,
    pub nonce: u128,
}

pub const OPTIONS: &str = "options";

// Set of pairs <TX_HASH, TX_DATA>
pub const TRANSACTIONS: &str = "transactions";

// Transaction queue
pub const TRANSACTION_HASHES: &str = "transaction_hashes";

// Directions
pub const DIRECTION_LEFT: &str = "LEFT";
pub const DIRECTION_RIGHT: &str = "RIGHT";

// Transaction queue
pub const EVENTS: &str = "events";

pub const PENDING_TRANSACTIONS: &str = "pending_transactions";

pub const ABOUT_TO_UNLOCK_TRANSACTIONS: &str = "about_to_proving_transactions";

// TODO: review. Moved from the redis_wrapper
const REDIS_TRANSACTION_HASH: &str = "myhash";
const REDIS_PROFIT_HASH: &str = "myprofit";

impl AsyncRedisWrapper {
    pub async fn connect(settings: std::sync::Arc<std::sync::Mutex<crate::Settings>>) -> Self {
        let redis_settings = settings.lock().unwrap().redis.clone();
        let client = redis::Client::open(redis_settings.url.clone())
            .expect("REDIS: Failed to establish connection");
        let connection = client
            .get_multiplexed_tokio_connection()
            .await
            .expect("REDIS: Failed to get connection");
        AsyncRedisWrapper { client, connection }
    }

    pub async fn option_set<T: redis::ToRedisArgs + Send + Sync>(
        &mut self,
        name: &str,
        value: T,
    ) -> redis::RedisResult<()> {
        self.connection.hset(OPTIONS, name, value).await?;
        Ok(())
    }

    pub async fn option_get<T: redis::ToRedisArgs + Send + Sync + redis::FromRedisValue>(
        &mut self,
        name: &str,
    ) -> redis::RedisResult<Option<T>> {
        let val: Option<T> = self.connection.hget(OPTIONS, name).await?;
        Ok(val)
    }

    pub async fn event_pub(&mut self, event: spectre_bridge_common::Event) {
        let _: () = self
            .connection
            .publish(EVENTS, serde_json::to_string(&event).unwrap())
            .await
            .unwrap();
    }

    pub async fn store_tx(
        &mut self,
        tx_hash: String,
        tx_data: TransactionData,
    ) -> redis::RedisResult<()> {
        let storing_status = self
            .hsetnx(
                TRANSACTIONS,
                &tx_hash,
                &serde_json::to_string(&tx_data)
                    .expect("REDIS: Failed to serialize transaction data"),
            )
            .await;
        if let Ok(redis::Value::Int(1)) = storing_status {
            return Ok(self.rpush(TRANSACTION_HASHES, &tx_hash).await?);
        } else {
            return Err(storing_status.unwrap_err());
        }
    }

    pub async fn unstore_tx(&mut self, tx_hash: String) -> redis::RedisResult<()> {
        let unstoring_status = self.hdel(TRANSACTIONS, &tx_hash).await;
        if let Ok(redis::Value::Int(1)) = unstoring_status {
            return Ok(self.lrem(TRANSACTION_HASHES, 0, &tx_hash).await?);
        } else {
            return Err(unstoring_status.unwrap_err());
        }
    }

    pub async fn get_tx_hash(&mut self) -> redis::RedisResult<Option<String>> {
        match self.lindex(TRANSACTION_HASHES, 0).await {
            Ok(value) => {
                if let redis::Value::Data(_) = value {
                    return Ok(Some(redis::from_redis_value(&value)?));
                } else {
                    return Ok(None);
                }
            }
            Err(error) => Err(error),
        }
    }

    pub async fn get_tx_data(&mut self, tx_hash: String) -> redis::RedisResult<TransactionData> {
        match self.hget(TRANSACTIONS, &tx_hash).await {
            Ok(value) => {
                let serialized_tx_data: String = redis::from_redis_value(&value)?;
                return Ok(serde_json::from_str(&serialized_tx_data)
                    .expect("REDIS: Failed to deserialize transaction data"));
            }
            Err(error) => Err(error),
        }
    }

    pub async fn move_tx_queue_tail(&mut self) -> redis::RedisResult<()> {
        Ok(self
            .lmove(
                TRANSACTION_HASHES,
                TRANSACTION_HASHES,
                DIRECTION_LEFT,
                DIRECTION_RIGHT,
            )
            .await?)
    }

    async fn hsetnx(
        &mut self,
        key: &str,
        field: &str,
        value: &str,
    ) -> redis::RedisResult<redis::Value> {
        Ok(self.connection.hset_nx(key, field, value).await?)
    }

    async fn rpush(&mut self, key: &str, value: &str) -> redis::RedisResult<()> {
        Ok(self.connection.rpush(key, value).await?)
    }

    async fn lindex(&mut self, key: &str, index: isize) -> redis::RedisResult<redis::Value> {
        Ok(self.connection.lindex(key, index).await?)
    }

    async fn lrem(&mut self, key: &str, count: isize, value: &str) -> redis::RedisResult<()> {
        Ok(self.connection.lrem(key, count, value).await?)
    }

    async fn hget(&mut self, key: &str, field: &str) -> redis::RedisResult<redis::Value> {
        Ok(self.connection.hget(key, field).await?)
    }

    async fn hdel(&mut self, key: &str, field: &str) -> redis::RedisResult<redis::Value> {
        Ok(self.connection.hdel(key, field).await?)
    }

    async fn lmove(
        &mut self,
        srckey: &str,
        dstkey: &str,
        src_dir: &str,
        dst_dir: &str,
    ) -> redis::RedisResult<()> {
        Ok(redis::cmd("lmove")
            .arg(srckey)
            .arg(dstkey)
            .arg(src_dir)
            .arg(dst_dir)
            .query_async(&mut self.connection)
            .await?)
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
            .hget(REDIS_PROFIT_HASH, "profit".to_string())
            .await
            .ok()
            .unwrap_or(0); // In case we don't have initial value in DB

        self.connection
            .hset(
                REDIS_PROFIT_HASH,
                "profit".to_string(),
                add_to + profit as u64,
            )
            .await?;

        Ok(())
    }

    // TODO: review. Moved from the redis_wrapper
    pub async fn get_profit(&mut self) -> u64 {
        self.connection
            .hget(REDIS_PROFIT_HASH, "profit".to_string())
            .await
            .ok()
            .unwrap()
    }
}

pub fn subscribe<T: 'static + redis::FromRedisValue + Send>(
    channel: String,
    redis: std::sync::Arc<std::sync::Mutex<AsyncRedisWrapper>>,
) -> RedisResult<tokio::sync::mpsc::Receiver<T>> {
    let (sender, receiver) = tokio::sync::mpsc::channel::<T>(100);
    tokio::spawn(async move {
        let client = redis.lock().unwrap().client.clone();
        let mut pubsub_connection = client
            .get_async_connection()
            .await
            .expect("REDIS: Failed to get connection")
            .into_pubsub();
        pubsub_connection
            .subscribe(channel)
            .await
            .expect("Failed to subscribe to the channel");
        let mut pubsub_stream = pubsub_connection.on_message();

        while let Some(s) = pubsub_stream.next().await {
            let pubsub_msg: T = s.get_payload().expect("Failed to fetch the message");
            if let Err(e) = sender.send(pubsub_msg).await {
                break;
            }
        }
    });
    Ok(receiver)
}
