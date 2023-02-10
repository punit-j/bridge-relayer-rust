use crate::config::SafeSettings;
use near_sdk::borsh::BorshDeserialize;

pub type SafeStorage = std::sync::Arc<tokio::sync::Mutex<Storage>>;

#[derive(Clone, Debug)]
pub struct Storage {
    pub eth_last_block_number_on_near: u64,
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            eth_last_block_number_on_near: 0,
        }
    }
}

pub async fn last_block_number_worker(settings: SafeSettings, storage: SafeStorage) {
    tokio::spawn(async move {
        loop {
            let last_block_number_worker_settings =
                settings.lock().await.last_block_number_worker.clone();

            let mut interval = crate::utils::request_interval(
                last_block_number_worker_settings.request_interval_secs,
            )
            .await;
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval.tick().await;
            let number = last_block_number(
                last_block_number_worker_settings.server_addr,
                last_block_number_worker_settings.contract_account_id,
            )
            .await;
            match number {
                Ok(result) => match result {
                    Some(block_number) => {
                        storage.lock().await.eth_last_block_number_on_near = block_number
                    }
                    None => (),
                },
                Err(error) => tracing::error!("{}", error),
            }
        }
    });
}

pub async fn last_block_number(
    server_addr: url::Url,
    contract_account_id: String,
) -> Result<Option<u64>, crate::errors::CustomError> {
    let response = near_client::methods::view(
        server_addr,
        contract_account_id,
        "last_block_number".to_string(),
        near_sdk::serde_json::json!({}),
    )
    .await;
    match response {
        Ok(response_result) => {
            if let near_jsonrpc_primitives::types::query::QueryResponseKind::CallResult(result) =
                response_result.kind
            {
                Ok(Some(u64::try_from_slice(&result.result).unwrap()))
            } else {
                Ok(None)
            }
        }
        Err(error) => Err(crate::errors::CustomError::FailedExecuteLastBlockNumber(
            error.to_string(),
        )),
    }
}

#[cfg(test)]
pub mod tests {
    use crate::last_block::{last_block_number, last_block_number_worker, Storage};
    use crate::logs::init_logger;
    use crate::test_utils::get_settings;
    use std::time::Duration;

    #[tokio::test]
    async fn smoke_last_block_number_test() {
        init_logger();
        let server_addr = url::Url::parse("https://rpc.testnet.near.org").unwrap();
        let contract_account_id = "client6.goerli.testnet".to_string();

        let last_block_number = last_block_number(server_addr, contract_account_id)
            .await
            .unwrap()
            .unwrap();
        println!("last_block_number = {}", last_block_number);
    }

    #[tokio::test]
    async fn smoke_last_block_number_worker_test() {
        init_logger();

        let settings = std::sync::Arc::new(tokio::sync::Mutex::new(get_settings()));

        let storage = std::sync::Arc::new(tokio::sync::Mutex::new(Storage::new()));

        let _last_block_worker = last_block_number_worker(settings.clone(), storage.clone()).await;
        tokio::time::sleep(Duration::from_secs(16)).await;

        let new_last_block_number = storage.clone().lock().await.eth_last_block_number_on_near;
        assert_ne!(new_last_block_number, 0u64);
        println!("new last block number = {}", new_last_block_number);
    }
}
