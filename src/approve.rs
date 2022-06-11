pub async fn approve(
    server_addr: &str,
    contract_addr: web3::types::Address,
    contract_abi: &[u8],
    spender: &str,
    amount: u128,
    key: impl web3::signing::Key,
) -> String {
    let spender: web3::types::H160 = spender.parse().unwrap();
    let amount = web3::types::U256::from(amount);
    let tx_hash = eth_client::methods::change(
        server_addr,
        contract_addr,
        contract_abi,
        "approve",
        (spender, amount),
        key,
    )
    .await
    .expect("Failed to execute approve method");
    format!("{:#?}", tx_hash)
}
