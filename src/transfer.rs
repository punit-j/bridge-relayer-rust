// Somewhere define and init mempool by calling get_all() of RedisWrapper -- let mut mempool = redis.get_all(); --
// loop poll_tx() to get and remove first tx in mempool
// pass as a second parameter to execute_transfer()

pub fn poll_tx(mempool: &mut Vec<String>) -> crate::transfer_event::SpectreBridgeTransferEvent {
    let event: Option<crate::transfer_event::SpectreBridgeTransferEvent> = serde_json::from_str(mempool.first().unwrap()).expect("Unable to parse JSON");
    mempool.remove(0);
    event.unwrap()
}

// For testing only (token: USDC)
async fn is_profitable(
    fee: crate::transfer_event::Transfer,
    estimated_transfer_execution_price: f64,
    profit_threshold: f64,
) -> bool {
    let precision = f64::powf(10.0, 4.0);
    let token_price = eth_client::methods::token_price(
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
            .parse()
            .unwrap(),
    )
    .await
    .expect("Failed to get token price");
    let token_price = web3::types::U256::from((token_price * precision) as u64);
    let fee_amount = web3::types::U256::from(fee.amount);
    let fee_amount_usd = token_price.checked_mul(fee_amount).unwrap().as_u64() as f64 / precision;
    fee_amount_usd - estimated_transfer_execution_price > profit_threshold
}

pub async fn execute_transfer(
    from: &str,
    private_key: &str,
    transfer_message: crate::transfer_event::SpectreBridgeTransferEvent,
    nonce: u128,
    contract_abi: &[u8],
    config: crate::config::Settings,
) -> String {
    let server_addr = config.eth_settings.rpc_url.as_str();
    let contract_addr = config.eth_settings.contract_address.as_str();
    let method_name = "transferTokens";
    let method_args = (
        transfer_message.transfer.token,
        transfer_message.recipient,
        web3::types::U256::from(nonce),
        transfer_message.transfer.amount,
    );
    let estimated_gas_in_wei = eth_client::methods::estimate_gas(
        server_addr,
        from,
        contract_abi,
        method_name,
        method_args,
    )
    .await
    .expect("Failed to estimate gas in WEI");
    let gas_price_in_wei = eth_client::methods::gas_price(server_addr)
        .await
        .expect("Failed to fetch gas price in WEI");
    let eth_price_in_usd = eth_client::methods::eth_price()
        .await
        .expect("Failed to fetch Ethereum price in USD");
    let estimated_transfer_execution_price = eth_client::methods::estimate_transfer_execution(
        estimated_gas_in_wei,
        gas_price_in_wei,
        eth_price_in_usd,
    );
    let profit_threshold = config.profit_thershold.lock().unwrap().to_owned() as f64;
    let is_profitable_tx = is_profitable(
        transfer_message.fee,
        estimated_transfer_execution_price,
        profit_threshold,
    )
    .await;
    match is_profitable_tx {
        true => {
            let tx_hash = eth_client::methods::change(
                server_addr,
                contract_addr,
                contract_abi,
                method_name,
                method_args,
                private_key,
            )
            .await
            .expect("Failed to execute tokens transfer");
            format!("{:#?}", tx_hash)
        }
        false => "".to_string(),
    }
}

// From: https://github.com/spectrebridge/spectre-bridge-protocol/blob/main/near/contracts/transfer/src/lp_relayer.rs#L28
pub struct Proof {
    pub log_index: u64,
    pub log_entry_data: Vec<u8>,
    pub receipt_index: u64,
    pub receipt_data: Vec<u8>,
    pub header_data: Vec<u8>,
    pub proof: Vec<Vec<u8>>,
}

pub fn generate_proof(tx_hash: String, block: web3::types::BlockNumber) -> Proof {
    Proof {
        log_index: 0,
        log_entry_data: vec![],
        receipt_index: 0,
        receipt_data: vec![],
        header_data: vec![],
        proof: vec![],
    }
}
