pub fn format_token_units(token_amount: web3::types::U256, decimals: u32) -> String {
    let decimals = 10u64.pow(decimals);
    let (amount_integer, amount_decimals) = token_amount.div_mod(decimals.into());
    format!(
        "{}.{:0width$}",
        amount_integer,
        amount_decimals,
        width = decimals as usize
    )
}

pub async fn get_profit_usd(
    fee_token_price_usd: f64,
    fee_token_amount: web3::types::U256,
    fee_token_decimals: u32,
    estimated_transfer_execution_price_usd: f64,
) -> Option<f64> {
    let readable_fee_token_amount = rug::Float::with_val(
        128,
        rug::Float::parse(format_token_units(fee_token_amount, fee_token_decimals)).ok()?,
    );

    Some(
        readable_fee_token_amount
            .mul_sub(
                &rug::Float::with_val(64, fee_token_price_usd),
                &rug::Float::with_val(64, estimated_transfer_execution_price_usd),
            )
            .to_f64(),
    )
}

#[cfg(test)]
pub mod tests {
    use crate::profit_estimation::get_profit_usd;

    #[tokio::test]
    async fn smoke_get_profit_test() {
        const EPS: f64 = 0.0001;
        let one_token_amount = 1_000_000;

        let profit_usd =
            get_profit_usd(0.5, web3::types::U256::from(10 * one_token_amount), 6, 2.0)
                .await
                .unwrap();
        assert!(profit_usd - 3.0 < EPS && 3.0 - profit_usd < EPS);
    }

    #[tokio::test]
    #[ignore]
    async fn super_cheap_token_test() {
        const EPS: f64 = 0.0001;
        let token_price_usd = 1. / 1_000_000_000.;
        let token_amount = web3::types::U256::from(2_000_000_000);

        let profit_usd = get_profit_usd(token_price_usd, token_amount, 0, 1.0)
            .await
            .unwrap();
        assert!(
            profit_usd - 1.0 < EPS && 1.0 - profit_usd < EPS,
            "Incorrect profit: expected = 1, found = {}",
            profit_usd
        );
    }

    #[tokio::test]
    async fn cheap_token_test() {
        const EPS: f64 = 0.0001;
        let token_price_usd = 1. / 10_000.;
        let token_amount = web3::types::U256::from(20_000);

        let profit_usd = get_profit_usd(token_price_usd, token_amount, 0, 1.0)
            .await
            .unwrap();
        assert!(
            profit_usd - 1.0 < EPS && 1.0 - profit_usd < EPS,
            "Incorrect profit: expected = 1, found = {}",
            profit_usd
        );
    }
}
