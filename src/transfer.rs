/// Usage:
/// somewhere define and init mempool by calling get_all() of RedisWrapper -- let mut mempool = redis.get_all(); --
/// loop poll_tx() to get and remove first tx in mempool
/// pass as a second parameter to execute_transfer()

pub fn poll_tx(mempool: &mut Vec<String>) -> crate::transfer_event::SpectreBridgeTransferEvent {
    let event: Option<crate::transfer_event::SpectreBridgeTransferEvent> = serde_json::from_str(mempool.first().unwrap()).expect("Unable to parse JSON");
    mempool.remove(0);
    event.unwrap()
}

pub async fn execute_transfer(config: crate::config::Settings, transfer_message: crate::transfer_event::SpectreBridgeTransferEvent) {
    let eth_server_addr = config.eth_settings.rpc_url.as_str();
    let eth_contract_addr = config.eth_settings.contract_address.as_str();
    let etherscan_endpoint_url = "https://api-rinkeby.etherscan.io"; // <--- Where to take
    let etherscan_api_key_token = "25PJD4MEHKXRJ8G1J8Z3HU32USC1SH1K3E";  // <--- Where to take
    let eth_contract_abi = eth_client::methods::get_contract_abi(etherscan_endpoint_url, eth_contract_addr, etherscan_api_key_token).await.expect("Failed to fetch contract abi");
    let eth_contract_abi = eth_contract_abi.as_bytes(); // <--- Where to take
    let eth_contract_method_name = "store";  // <--- Where to take
    let eth_contract_method_args = 999_u32;  // <--- Where to take
    let estimated_gas_in_wei = eth_client::methods::estimate_gas(eth_server_addr, eth_contract_addr, eth_contract_abi, eth_contract_method_name, eth_contract_method_args).await.expect("Failed to estimate gas in WEI");
    let gas_price_in_wei = eth_client::methods::gas_price(eth_server_addr).await.expect("Failed to fetch gas price in WEI");
    let ether_price_in_usd = eth_client::methods::eth_price().await.expect("Failed to fetch Ethereum price in USD");
    let estimated_transfer_execution_price = eth_client::methods::estimate_transfer_execution(estimated_gas_in_wei, gas_price_in_wei, ether_price_in_usd);
    let profit_threshold = config.profit_thershold.lock().unwrap().to_owned() as f64;
    let is_profitable_tx = crate::profit_estimation::is_profitable(transfer_message.transfer, estimated_transfer_execution_price, profit_threshold).await;
    match is_profitable_tx {
        true => {
            let eth_private_key = config.eth_settings.private_key.as_str();
            eth_client::methods::change(eth_server_addr, eth_contract_addr, eth_contract_abi, eth_contract_method_name, eth_contract_method_args, eth_private_key).await.expect("Failed to execute contract call");
            // TODO: Generate proof
            let near_server_addr = config.near_settings.rpc_url.as_str();
            let signer_account_id = "arseniyrest.testnet";  // <--- Where to take
            let signer_secret_key = config.near_settings.private_key.as_str();
            let contract_address = config.near_settings.contract_address.as_str();
            let nonce = 999;  // <--- Where to take
            let gas = 100_000_000_000_000;  // <--- Where to take
            let unlock_tokens_status = crate::unlock_tokens::unlock_tokens(near_server_addr, signer_account_id, signer_secret_key, contract_address, nonce, gas).await;
            // TODO: handle call unclock() contract method --> unlock_tokens_status
        },
        false => {
            // TODO: actions if transaction does not profitable
        },
    }
}
