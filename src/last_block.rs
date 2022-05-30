pub struct Storage {
    pub last_block_number: std::sync::Mutex<u64>,
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            last_block_number: std::sync::Mutex::new(0),
        }
    }
}

pub async fn last_block_number_worker(
    seconds: u64,
    server_addr: String,
    contract_account_id: String,
    storage: std::sync::Arc<std::sync::Mutex<Storage>>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(seconds));
        loop {
            let number = last_block_number(server_addr.clone(), contract_account_id.clone())
                .await
                .expect("Failed to fetch block number");
            {
                let mut storage = storage.lock().unwrap();
                storage.last_block_number = std::sync::Mutex::new(number);
            }
            interval.tick().await;
        }
    });
}

async fn last_block_number(
    server_addr: String,
    contract_account_id: String,
) -> Result<u64, near_sdk::serde_json::Error> {
    let response = near_client::methods::view(
        server_addr,
        contract_account_id,
        "last_block_number".to_string(),
        near_sdk::serde_json::json!({}),
    )
    .await;
    if let near_jsonrpc_primitives::types::query::QueryResponseKind::CallResult(result) =
        response.unwrap().kind
    {
        return Ok(near_sdk::serde_json::from_slice::<u64>(&result.result)?);
    };
    panic!("Critical error while fetching last block number")
}

#[cfg(test)]
pub mod tests {

    const SERVER_ADDR: &str = "https://rpc.testnet.near.org";
    const CONTRACT_ACCOUNT_ID: &str = "client6.goerli.testnet";
    const LATEST_BLOCK_WORKER_DELAY: u64 = 15;

    pub async fn get_last_block_number(
        storage: std::sync::Arc<std::sync::Mutex<super::Storage>>,
    ) -> u64 {
        *storage.lock().unwrap().last_block_number.lock().unwrap()
    }

    #[tokio::test]
    async fn query_latest_block() {
        let response =
            super::last_block_number(SERVER_ADDR.to_string(), CONTRACT_ACCOUNT_ID.to_string())
                .await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn store_last_block_number() {
        let storage = std::sync::Arc::new(std::sync::Mutex::new(super::Storage::new()));
        let current_block_number = get_last_block_number(storage.clone()).await;
        super::last_block_number_worker(
            LATEST_BLOCK_WORKER_DELAY,
            SERVER_ADDR.to_string(),
            CONTRACT_ACCOUNT_ID.to_string(),
            storage.clone(),
        )
        .await;
        {
            // Mocked waiting for a new block to finish the test correctly
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(LATEST_BLOCK_WORKER_DELAY));
            interval.tick().await;
        }
        let new_block_number = get_last_block_number(storage.clone()).await;
        assert_eq!(current_block_number, new_block_number);
    }
}
