pub struct Storage {
    block: std::sync::Mutex<web3::types::Block<web3::types::H256>>,
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            block: std::sync::Mutex::new(web3::types::Block::default()),
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
            let number = last_block_number(server_addr.clone(), contract_account_id.clone()).await;
            let latest_block =
                eth_client::methods::block(&server_addr, web3::types::BlockNumber::Latest)
                    .await
                    .expect("Failed to get latest block");
            match latest_block.number.unwrap().as_u64() < number {
                true => {
                    let block = eth_client::methods::block(
                        &server_addr,
                        web3::types::BlockNumber::Number(web3::types::U64::from(number)),
                    )
                    .await
                    .expect("Failed to get block by number");
                    {
                        let mut storage = storage.lock().unwrap();
                        storage.block = std::sync::Mutex::new(block);
                    }
                }
                false => (),
            }

            interval.tick().await;
        }
    });
}

async fn last_block_number(server_addr: String, contract_account_id: String) -> u64 {
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
        return near_sdk::serde_json::from_slice::<u64>(&result.result)
            .expect("Failed to get last block number");
    };
    0
}
