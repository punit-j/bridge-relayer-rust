use redis::{AsyncCommands, RedisResult};

#[derive(Clone)]
pub struct AsyncRedisWrapper {
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

impl AsyncRedisWrapper {
    pub async fn connect(settings: crate::config::RedisSettings) -> Self {
        let client = redis::Client::open(settings.url.clone())
            .expect("REDIS: Failed to establish connection");
        let connection = client
            .get_multiplexed_tokio_connection()
            .await
            .expect("REDIS: Failed to get connection");
        AsyncRedisWrapper { connection }
    }

    pub async fn option_set<T: redis::ToRedisArgs + Send + Sync>(&mut self, name: &str, value: T) -> redis::RedisResult<()> {
        self.connection.hset(OPTIONS, name, value).await?;
        Ok(())
    }

    pub async fn option_get<T: redis::ToRedisArgs + Send + Sync + redis::FromRedisValue>(&mut self, name: &str) -> redis::RedisResult<Option<T>> {
        let val: Option<T> = self.connection.hget(OPTIONS, name).await?;
        Ok(val)
    }

    pub async fn event_push(&mut self, event: spectre_bridge_common::Event) {
        let _: () = self.connection.rpush(EVENTS, serde_json::to_string(&event).unwrap()).await.unwrap();
    }

    pub async fn event_pop(&mut self, event: spectre_bridge_common::Event) -> Result<spectre_bridge_common::Event, String> {
        let r: String = self.connection.lpop(EVENTS, None).await.map_err(|e| e.to_string())?;
        let event = serde_json::from_str::<spectre_bridge_common::Event>(&r).map_err(|e| e.to_string())?;
        Ok(event)
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
}
