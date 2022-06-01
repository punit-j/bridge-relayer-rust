use redis::AsyncCommands;
use futures_util::StreamExt;

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

// Set of pairs <TX_HASH, TX_DATA>
const TRANSACTIONS: &str = "transactions";

// Transaction queue
const TRANSACTION_HASHES: &str = "transaction_hashes";

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
