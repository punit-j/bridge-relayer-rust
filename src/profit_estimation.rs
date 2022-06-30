pub async fn get_profit(
    fee_token_usd: f64,
    fee_amount: web3::types::U256,
    estimated_transfer_execution_price: f64,
) -> f64 {
    let precision = f64::powf(10.0, 4.0);
    let token_price = web3::types::U256::from((fee_token_usd * precision) as u64);
    let fee_amount_usd = token_price.checked_mul(fee_amount).unwrap().as_u128() as f64 / precision;
    fee_amount_usd - estimated_transfer_execution_price
}
