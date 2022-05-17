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
            println!("{}", number);
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
