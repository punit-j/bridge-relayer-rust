pub async fn get_profit(
    fee_token_price_usd: f64,
    fee_token_amount: web3::types::U256,
    estimated_transfer_execution_price_usd: f64,
) -> f64 {
    let precision = f64::powf(10.0, 4.0);
    let fee_token_price_usd = web3::types::U256::from((fee_token_price_usd * precision) as u128);
    let total_fee_price_usd = fee_token_price_usd
        .checked_mul(fee_token_amount)
        .unwrap()
        .as_u128() as f64
        / precision;
    total_fee_price_usd - estimated_transfer_execution_price_usd
}
