pub async fn unlock_tokens(
    server_addr: &str,
    signer_account_id: &str,
    signer_secret_key: &str,
    contract_address: &str,
    nonce: u128,
    gas: u64,
) -> near_primitives::views::FinalExecutionStatus {
    let response = near_client::methods::change(
        server_addr.to_string(), 
        signer_account_id.to_string(), 
        signer_secret_key.to_string(), 
        contract_address.to_string(), 
        "unlock".to_string(), 
        near_sdk::serde_json::json!({
            "nonce": near_sdk::json_types::U128(nonce),
        }), 
        gas, 
        0, 
    ).await.expect("Failed to get response by calling unlock() contract method");
    response.status
}

#[cfg(test)]
pub mod tests {

    #[tokio::test]
    // Currently the status will be Failed, because "response" returns Failure
    pub async fn unlock_tokens() {
        let response = super::unlock_tokens(
            "https://rpc.testnet.near.org", 
            "arseniyrest.testnet", 
            near_client::read_private_key::read_private_key_from_file(
                "/home/arseniyk/.near-credentials/testnet/arseniyrest.testnet.json",
            ).as_str(), 
            "transfer.spectrebridge.testnet", 
            1, 
            100_000_000_000_000, 
        )
        .await;
        assert_eq!(response.as_success().is_some(), true); 
    }
}