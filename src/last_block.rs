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
