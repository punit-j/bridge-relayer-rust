pub async fn execute_transfer(
    key: impl web3::signing::Key,
    transfer_message: spectre_bridge_common::Event,
    contract_abi: &[u8],
    rpc_url: &str,
    contract_addr: web3::types::Address,
    profit_threshold: f64,
    near_tokens_coin_id: &crate::config::NearTokensCoinId
) -> Result<web3::types::H256, String> {
    let method_name = "transferTokens";
    let transfer_message = if let spectre_bridge_common::Event::SpectreBridgeTransferEvent {
        nonce,
        chain_id,
        valid_till,
        transfer,
        fee,
        recipient,
    } = transfer_message
    {
        (nonce, chain_id, valid_till, transfer, fee, recipient)
    } else {
        return Err("Incorrect event passed".to_string());
    };

    let token = web3::types::Address::from(transfer_message.3.token_eth);
    let recipient = web3::types::Address::from(transfer_message.5);
    let nonce = web3::types::U256::from(transfer_message.0 .0);
    let amount = web3::types::U256::from(transfer_message.3.amount.0);
    let method_args = (token, recipient, nonce, amount);
    let estimated_gas_in_wei = eth_client::methods::estimate_gas(
        rpc_url,
        key.address(),
        contract_abi,
        method_name,
        method_args,
    )
        .await
        .expect("Failed to estimate gas in WEI");
    let gas_price_in_wei = eth_client::methods::gas_price(rpc_url)
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

    let fee_token = transfer_message.4.token;
    let coin_id = near_tokens_coin_id.get_coin_id(fee_token).expect("Failed to get coin id by matching");
    let fee_amount = web3::types::U256::from(transfer_message.4.amount.0);
    let is_profitable_tx = crate::profit_estimation::is_profitable(
        coin_id,
        fee_amount,
        estimated_transfer_execution_price,
        profit_threshold,
    )
        .await;

    if !is_profitable_tx {
        return Err("transaction is not profitable".to_string());
    }

    let tx_hash = eth_client::methods::change(
        rpc_url,
        contract_addr,
        contract_abi,
        method_name,
        method_args,
        key,
    ).await.map_err(|e| format!("Failed to execute tokens transfer: {}", e.to_string()))?;
    Ok(tx_hash)
}
