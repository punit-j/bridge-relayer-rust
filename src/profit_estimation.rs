pub async fn is_profitable(
    coin_id: String,
    fee_amount: web3::types::U256,
    estimated_transfer_execution_price: f64,
    profit_threshold: f64,
) -> bool {
    let precision = f64::powf(10.0, 4.0);
    let token_price = eth_client::methods::token_price(coin_id)
        .await
        .expect("Failed to get token price");
    let token_price = web3::types::U256::from((token_price * precision) as u64);
    let fee_amount_usd = token_price.checked_mul(fee_amount).unwrap().as_u64() as f64 / precision;
    fee_amount_usd - estimated_transfer_execution_price > profit_threshold
}
