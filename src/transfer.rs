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

#[cfg(test)]
pub mod tests {

    const ETH_RPC_ENDPOINT_URL: &str =
        "https://goerli.infura.io/v3/ba5fd6c86e5c4e8c9b36f3f5b4013f7a";
    const ETHERSCAN_RPC_ENDPOINT_URL: &str = "https://api-goerli.etherscan.io";

    #[tokio::test]
    async fn execute_transfer() {
        let from = "0x87b1fF03B64Fe4Bd063d8c6F7A01357FBEEdD51b";
        let private_key = "ebefaa0570e26ce96cf0876ff68648027de39b30119b16953aa93e73d35064c1";
        let transfer_message = crate::transfer_event::SpectreBridgeTransferEvent {
            valid_till: 54321,
            transfer: crate::transfer_event::Transfer {
                token: "0xb2d75C5a142A68BDA438e6a318C7FBB2242f9693"
                    .parse()
                    .unwrap(),
                amount: 100,
            },
            fee: crate::transfer_event::Transfer {
                token: "0xb2d75C5a142A68BDA438e6a318C7FBB2242f9693"
                    .parse()
                    .unwrap(),
                amount: 1,
            },
            recipient: "0x87b1fF03B64Fe4Bd063d8c6F7A01357FBEEdD51b"
                .parse()
                .unwrap(),
        };

        let contract_addr = "0x5c739e4039D552E2DBF94ce9E7Db261c88BcEc84";

        let contract_abi = eth_client::methods::get_contract_abi(
            ETHERSCAN_RPC_ENDPOINT_URL,
            contract_addr,
            "",
        )
        .await;
        assert!(contract_abi.is_ok());
        let contract_abi = contract_abi.unwrap();
        assert!(!contract_abi.is_empty());

        let method_name = "transferTokens";
        let method_args = (
            transfer_message.transfer.token,
            transfer_message.recipient,
            web3::types::U256::from(12345),
            transfer_message.transfer.amount,
        );

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
            transfer_message.fee,
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
