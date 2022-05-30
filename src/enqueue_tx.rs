use redis::AsyncCommands;

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TxData {
    block: u64,
    proof: spectre_bridge_common::Proof,
}

const TRANSACTIONS: &str = "txs";
const TRANSACTION_QUEUE: &str = "txqueue";

pub struct RedisWrapper {
    pub connection: redis::aio::Connection,
}

impl RedisWrapper {
    pub async fn connect(settings: crate::config::RedisSettings) -> redis::RedisResult<Self> {
        let client = redis::Client::open(settings.url.clone())?;
        let connection = client.get_async_connection().await?;
        Ok(RedisWrapper { connection })
    }

    pub async fn hset(&mut self, tx_hash: String, tx_data: TxData) -> redis::RedisResult<()> {
        self.connection
            .hset(
                TRANSACTIONS,
                &tx_hash,
                serde_json::to_string(&tx_data).expect(""),
            )
            .await?;
        self.connection.rpush(TRANSACTION_QUEUE, tx_hash).await?;
        Ok(())
    }

    pub async fn hget(&mut self, tx_hash: String) -> redis::RedisResult<TxData> {
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
            .lpop(TRANSACTION_QUEUE, std::num::NonZeroUsize::new(1))
            .await?;
        match tx_hashes.is_empty() {
            true => Ok(None),
            false => Ok(Some(tx_hashes[0].clone())),
        }
    }

    pub async fn rpush(&mut self, tx_hash: String) -> redis::RedisResult<()> {
        Ok(self.connection.rpush(TRANSACTION_QUEUE, tx_hash).await?)
    }
}

pub async fn unlock_tokens_worker(
    server_addr: String,
    signer_account_id: String,
    signer_secret_key: String,
    contract_address: String,
    gas: u64,
    seconds: u64,
    some_number: u64,
    settings: crate::config::RedisSettings,
    storage: std::sync::Arc<std::sync::Mutex<crate::last_block::Storage>>,
) -> redis::RedisResult<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(seconds));
        let mut redis = RedisWrapper::connect(settings).await.expect("");
        loop {
            match redis.lpop().await.expect("") {
                Some(tx_hash) => {
                    let mut last_block_number: u64;
                    {
                        let mut storage = storage.lock().unwrap();
                        last_block_number = *storage.last_block_number.lock().unwrap();
                    }
                    let tx_data = redis.hget(tx_hash.clone()).await.expect("");
                    match tx_data.block + some_number <= last_block_number {
                        true => {
                            crate::unlock_tokens::unlock_tokens(
                                server_addr.clone(),
                                signer_account_id.clone(),
                                signer_secret_key.clone(),
                                contract_address.clone(),
                                tx_data.proof,
                                1,
                                gas,
                            )
                            .await;
                            redis.hdel(tx_hash.clone()).await.expect("");
                        }
                        false => redis.rpush(tx_hash.clone()).await.expect(""),
                    }
                }
                None => (),
            }
            interval.tick().await;
        }
    });
    Ok(())
}
