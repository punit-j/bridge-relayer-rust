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
    let is_profitable_tx = crate::profit_estimation::is_profitable(
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
