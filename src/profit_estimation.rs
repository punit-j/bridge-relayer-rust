pub async fn is_profitable(
    fee: crate::transfer_event::Transfer,
    estimated_transfer_execution_price: f64,
    profit_threshold: f64,
) -> bool {
    let precision = f64::powf(10.0, 4.0);
    // For testing only (mocked token_addr: USDC)
    let token_price = eth_client::methods::token_price(
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
            .parse()
            .unwrap(),
    )
    .await
    .expect("Failed to get token price");
    // Replace with previous after testing
    // let token_price = eth_client::methods::token_price(fee.token)
    //     .await
    //     .expect("Failed to get token price");
    let token_price = web3::types::U256::from((token_price * precision) as u64);
    let fee_amount = web3::types::U256::from(fee.amount);
    let fee_amount_usd = token_price.checked_mul(fee_amount).unwrap().as_u64() as f64 / precision;
    fee_amount_usd - estimated_transfer_execution_price > profit_threshold
}

#[cfg(test)]
pub mod tests {

    #[tokio::test]
    pub async fn is_profitable() {
        let contract_abi = eth_client::methods::get_contract_abi(
            "https://api-rinkeby.etherscan.io",
            "0x8d9Eda359157594F352dc29c0bDB741bb8F6b65e",
            "",
        )
        .await
        .expect("Failed to get contract abi");

        let estimated_gas_in_wei = eth_client::methods::estimate_gas(
            "https://rinkeby.infura.io/v3/168bdff2f03e417eb8e69cd90fc54615",
            "0x8d9Eda359157594F352dc29c0bDB741bb8F6b65e",
            contract_abi.as_bytes(),
            "store",
            0_u32,
        )
        .await
        .expect("Failde to estimate gas in wei");

        let gas_price_in_wei = eth_client::methods::gas_price(
            "https://rinkeby.infura.io/v3/168bdff2f03e417eb8e69cd90fc54615",
        )
        .await
        .expect("Failed to get gas price in wei");

        let ether_in_usd = eth_client::methods::eth_price()
            .await
            .expect("Failed to get ether price in usd");

        assert_eq!(
            true,
            super::is_profitable(
                crate::transfer_event::Transfer {
                    token: "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984"
                        .parse()
                        .unwrap(),
                    amount: 1000
                },
                eth_client::methods::estimate_transfer_execution(
                    estimated_gas_in_wei,
                    gas_price_in_wei,
                    ether_in_usd
                ),
                2.0
            )
            .await
        );
    }
}
