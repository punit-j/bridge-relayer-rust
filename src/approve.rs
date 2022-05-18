pub async fn approve(
    server_addr: &str,
    contract_addr: &str,
    contract_abi: &[u8],
    spender: &str,
    amount: u128,
    private_key: &str,
) -> String {
    let spender: web3::types::H160 = spender.parse().unwrap();
    let amount = web3::types::U256::from(amount);
    let tx_hash = eth_client::methods::change(
        server_addr,
        contract_addr,
        contract_abi,
        "approve",
        (spender, amount),
        private_key,
    )
    .await
    .expect("Failed to execute approve method");
    format!("{:#?}", tx_hash)
}
