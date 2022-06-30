use near_sdk::borsh::BorshDeserialize;

#[derive(Clone, Debug)]
pub struct Storage {
    pub last_block_number: u64,
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            last_block_number: 0,
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
                settings.lock().unwrap().clone().last_block_number_worker;
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
                    Some(block_number) => storage.lock().unwrap().last_block_number = block_number,
                    None => (),
                },
                Err(error) => eprintln!("{}", error),
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
