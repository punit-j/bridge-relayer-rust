pub async fn execute_transfer(
    from: &str,
    private_key: &str,
    transfer_message: spectre_bridge_common::Event,
    contract_abi: &[u8],
    rpc_url: &str,
    contract_addr: &str,
    profit_threshold: f64
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
        from,
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
    //let profit_threshold = config.profit_thershold.lock().unwrap().to_owned() as f64;
    let is_profitable_tx = crate::profit_estimation::is_profitable(
        token,
        amount,
        estimated_transfer_execution_price,
        profit_threshold,
    )
        .await;
    match is_profitable_tx {
        true => {
            let tx_hash = eth_client::methods::change(
                rpc_url,
                contract_addr,
                contract_abi,
                method_name,
                method_args,
                private_key,
            ).await.map_err(|e| format!("Failed to execute tokens transfer: {}", e.to_string()))?;
            Ok(tx_hash)
        }
        false => Err("is_profitable_tx is false".to_string()),
    }
}

#[cfg(test)]
pub mod tests {

    use std::str::FromStr;

    const ETH_RPC_ENDPOINT_URL: &str =
        "https://goerli.infura.io/v3/ba5fd6c86e5c4e8c9b36f3f5b4013f7a";
    const ETHERSCAN_RPC_ENDPOINT_URL: &str = "https://api-goerli.etherscan.io";

    #[tokio::test]
    async fn execute_transfer() {
        let from = "0x87b1fF03B64Fe4Bd063d8c6F7A01357FBEEdD51b";
        let private_key = "ebefaa0570e26ce96cf0876ff68648027de39b30119b16953aa93e73d35064c1";

        let transfer_message = spectre_bridge_common::Event::SpectreBridgeTransferEvent {
            nonce: near_sdk::json_types::U128(979797),
            chain_id: 0,
            valid_till: 0,
            transfer: spectre_bridge_common::TransferDataEthereum {
                token_near: near_sdk::AccountId::from_str(&"token".to_string()).unwrap(),
                token_eth: web3::types::H160::from_str("0xb2d75C5a142A68BDA438e6a318C7FBB2242f9693")
                    .unwrap()
                    .0,
                amount: near_sdk::json_types::U128(1),
            },
            fee: spectre_bridge_common::TransferDataNear {
                token: near_sdk::AccountId::from_str(&"token".to_string()).unwrap(),
                amount: 0.into(),
            },
            recipient: web3::types::H160::from_str("0x87b1fF03B64Fe4Bd063d8c6F7A01357FBEEdD51b")
                .unwrap()
                .0,
        };

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
            panic!("Incorrect event passed")
        };
        let token = web3::types::Address::from(transfer_message.3.token_eth);
        let recipient = web3::types::Address::from(transfer_message.5);
        let nonce = web3::types::U256::from(transfer_message.0 .0);
        let amount = web3::types::U256::from(transfer_message.3.amount.0);

        let contract_addr = "0x5c739e4039D552E2DBF94ce9E7Db261c88BcEc84";

        let contract_abi =
            eth_client::methods::get_contract_abi(ETHERSCAN_RPC_ENDPOINT_URL, contract_addr, "")
                .await;
        assert!(contract_abi.is_ok());
        let contract_abi = contract_abi.unwrap();
        assert!(!contract_abi.is_empty());

        let method_name = "transferTokens";
        let method_args = (token, recipient, nonce, amount);

        let estimated_gas_in_wei = eth_client::methods::estimate_gas(
            ETH_RPC_ENDPOINT_URL,
            from,
            contract_abi.as_bytes(),
            method_name,
            method_args,
        )
            .await
            .expect("Failed to estimate gas in WEI");
        assert_ne!(estimated_gas_in_wei, web3::types::U256::from(0));

        let gas_price_in_wei = eth_client::methods::gas_price(ETH_RPC_ENDPOINT_URL)
            .await
            .expect("Failed to fetch gas price in WEI");
        assert_ne!(gas_price_in_wei, web3::types::U256::from(0));

        let eth_price_in_usd = eth_client::methods::eth_price()
            .await
            .expect("Failed to fetch Ethereum price in USD");
        assert_ne!(eth_price_in_usd, 0.0);

        let estimated_transfer_execution_price = eth_client::methods::estimate_transfer_execution(
            estimated_gas_in_wei,
            gas_price_in_wei,
            eth_price_in_usd,
        );
        assert_ne!(estimated_transfer_execution_price, 0.0);

        let profit_threshold = 1.0;
        assert_ne!(profit_threshold, 0.0);

        let is_profitable_tx = crate::profit_estimation::is_profitable(
            token,
            amount,
            estimated_transfer_execution_price,
            profit_threshold,
        )
            .await;
        let result = match is_profitable_tx {
            true => {
                let tx_hash = eth_client::methods::change(
                    ETH_RPC_ENDPOINT_URL,
                    contract_addr,
                    contract_abi.as_bytes(),
                    method_name,
                    method_args,
                    private_key,
                )
                    .await;
                assert!(tx_hash.is_ok());
                format!("{:#?}", tx_hash)
            }
            false => "".to_string(),
        };
    }
}
