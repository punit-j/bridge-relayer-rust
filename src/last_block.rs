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
    settings: std::sync::Arc<std::sync::Mutex<crate::Settings>>,
    storage: std::sync::Arc<std::sync::Mutex<Storage>>,
) {
    tokio::spawn(async move {
        loop {
            let last_block_number_worker_settings =
                settings.lock().unwrap().last_block_number_worker.clone();
            crate::utils::request_interval(last_block_number_worker_settings.request_interval_secs)
                .await
                .tick()
                .await;
            let number = last_block_number(
                last_block_number_worker_settings.server_addr.clone(),
                last_block_number_worker_settings
                    .contract_account_id
                    .clone(),
            )
            .await
            .expect("Failed to fetch result by calling last_block_number view contract method");
            {
                let mut storage = storage.lock().unwrap();
                storage.last_block_number = std::sync::Mutex::new(number);
            }
        }
    });
}

pub async fn last_block_number(server_addr: url::Url, contract_account_id: String) -> Option<u64> {
    let response = near_client::methods::view(
        server_addr,
        contract_account_id,
        "last_block_number".to_string(),
        near_sdk::serde_json::json!({}),
    )
    .await
    .expect("Failed to fetch response by calling last_block_number contract method");
    if let near_jsonrpc_primitives::types::query::QueryResponseKind::CallResult(result) =
        response.kind
    {
        Some(u64::try_from_slice(&result.result).unwrap())
    } else {
        None
    }
}

#[cfg(test)]
pub mod tests {

    pub async fn query_storage_data(
        storage: std::sync::Arc<std::sync::Mutex<super::Storage>>,
    ) -> u64 {
        *storage.lock().unwrap().last_block_number.lock().unwrap()
    }

    pub async fn mocked_last_block_number_worker(
        server_addr: url::Url,
        contract_account_id: String,
        storage: std::sync::Arc<std::sync::Mutex<super::Storage>>,
    ) {
        let number = super::last_block_number(server_addr.clone(), contract_account_id.clone())
            .await
            .expect("Failed to fetch response by calling last_block_number contract method");
        let mut storage = storage.lock().unwrap();
        storage.last_block_number = std::sync::Mutex::new(number);
    }

    #[tokio::test]
    pub async fn query_last_block_number() {
        let number = super::last_block_number(
            url::Url::parse("https://rpc.testnet.near.org").unwrap(),
            "client6.goerli.testnet".to_string(),
        )
        .await;
        assert!(number.is_some());
        assert!(number.unwrap() > 0);
    }

    #[tokio::test]
    pub async fn store_last_block_number() {
        let storage = std::sync::Arc::new(std::sync::Mutex::new(super::Storage::new()));
        let initial_last_block_number = query_storage_data(storage.clone()).await;
        mocked_last_block_number_worker(
            url::Url::parse("https://rpc.testnet.near.org").unwrap(),
            "client6.goerli.testnet".to_string(),
            storage.clone(),
        )
        .await;
        let current_last_block_number = query_storage_data(storage.clone()).await;
        assert!(initial_last_block_number != current_last_block_number);
    }
}
