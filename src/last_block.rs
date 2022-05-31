use near_sdk::borsh::BorshDeserialize;

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
    server_addr: String,
    signer_account_id: String,
    signer_secret_key: String,
    contract_account_id: String,
    gas: u64,
    request_interval_sec: u64,
    storage: std::sync::Arc<std::sync::Mutex<Storage>>,
) {
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(request_interval_sec));
        loop {
            let number = last_block_number(
                server_addr.clone(),
                signer_account_id.clone(),
                signer_secret_key.clone(),
                contract_account_id.clone(),
                gas,
            )
            .await;
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
    signer_account_id: String,
    signer_secret_key: String,
    contract_account_id: String,
    gas: u64,
) -> u64 {
    let response = near_client::methods::change(
        server_addr,
        signer_account_id,
        signer_secret_key,
        contract_account_id,
        "last_block_number".to_string(),
        near_sdk::serde_json::json!({}),
        gas,
        0,
    )
    .await
    .expect("Failed to fetch response by calling last_block_number contract method");
    let base64_decoded_result =
        near_primitives::serialize::from_base64(&response.status.as_success().unwrap()).unwrap();
    u64::try_from_slice(&base64_decoded_result).unwrap()
}

#[cfg(test)]
pub mod tests {

    pub async fn query_storage_data(
        storage: std::sync::Arc<std::sync::Mutex<super::Storage>>,
    ) -> u64 {
        *storage.lock().unwrap().last_block_number.lock().unwrap()
    }

    pub async fn mocked_last_block_number_worker(
        server_addr: String,
        signer_account_id: String,
        signer_secret_key: String,
        contract_account_id: String,
        gas: u64,
        storage: std::sync::Arc<std::sync::Mutex<super::Storage>>,
    ) {
        let number = super::last_block_number(
            server_addr.clone(),
            signer_account_id.clone(),
            signer_secret_key.clone(),
            contract_account_id.clone(),
            gas,
        )
        .await;
        let mut storage = storage.lock().unwrap();
        storage.last_block_number = std::sync::Mutex::new(number);
    }

    #[tokio::test]
    pub async fn query_last_block_number() {
        let number = super::last_block_number(
            "https://rpc.testnet.near.org".to_string(),
            "arseniyrest.testnet".to_string(),
            near_client::read_private_key::read_private_key_from_file(
                "/home/arseniyk/.near-credentials/testnet/arseniyrest.testnet.json",
            ),
            "client6.goerli.testnet".to_string(),
            100_000_000_000_000,
        )
        .await;
        assert!(number > 0);
    }

    #[tokio::test]
    pub async fn store_last_block_number() {
        let storage = std::sync::Arc::new(std::sync::Mutex::new(super::Storage::new()));
        let initial_last_block_number = query_storage_data(storage.clone()).await;
        mocked_last_block_number_worker(
            "https://rpc.testnet.near.org".to_string(),
            "arseniyrest.testnet".to_string(),
            near_client::read_private_key::read_private_key_from_file(
                "/home/arseniyk/.near-credentials/testnet/arseniyrest.testnet.json",
            ),
            "client6.goerli.testnet".to_string(),
            100_000_000_000_000,
            storage.clone(),
        )
        .await;
        let current_last_block_number = query_storage_data(storage.clone()).await;
        assert!(initial_last_block_number != current_last_block_number);
    }
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
