use crate::config::Decimals;

pub fn format_token_units(token_amount: web3::types::U256, decimals: Decimals) -> String {
    let one_token_amount = 10u128.pow(decimals.clone().into());
    let (amount_integer, amount_decimals) = token_amount.div_mod(one_token_amount.into());
    format!(
        "{}.{:0width$}",
        amount_integer,
        amount_decimals,
        width = decimals.try_into().unwrap(),
    )
}

pub fn get_profit_usd(
    fee_token_price_usd: f64,
    fee_token_amount: web3::types::U256,
    fee_token_decimals: Decimals,
    estimated_transfer_execution_price_usd: f64,
) -> Result<f64, crate::errors::CustomError> {
    let fee_amount = if let Ok(val) = rug::Float::parse(format_token_units(
        fee_token_amount,
        fee_token_decimals.clone(),
    )) {
        val
    } else {
        return Err(crate::errors::CustomError::ErrorInFeeAmountParsing(
            fee_token_amount,
            fee_token_decimals.into(),
        ));
    };

    let readable_fee_token_amount = rug::Float::with_val(128, fee_amount);

    Ok(readable_fee_token_amount
        .mul_sub(
            &rug::Float::with_val(64, fee_token_price_usd),
            &rug::Float::with_val(64, estimated_transfer_execution_price_usd),
        )
        .to_f64())
}

#[cfg(test)]
pub mod tests {
    use crate::config::Decimals;
    use crate::profit_estimation::{format_token_units, get_profit_usd};

    #[test]
    fn smoke_get_profit_test() {
        const EPS: f64 = 0.0001;
        let one_token_amount = 1_000_000;

        let profit_usd = get_profit_usd(
            0.5,
            web3::types::U256::from(10 * one_token_amount),
            Decimals::try_from(6).unwrap(),
            2.0,
        )
        .unwrap();

        assert!(profit_usd - 3.0 < EPS && 3.0 - profit_usd < EPS);
    }

    #[test]
    #[ignore]
    fn super_cheap_token_test() {
        const EPS: f64 = 0.0001;
        let token_price_usd = 1. / 1_000_000_000.;
        let token_amount = web3::types::U256::from(2_000_000_000);

        let profit_usd = get_profit_usd(
            token_price_usd,
            token_amount,
            Decimals::try_from(0).unwrap(),
            1.0,
        )
        .unwrap();

        assert!(
            profit_usd - 1.0 < EPS && 1.0 - profit_usd < EPS,
            "Incorrect profit: expected = 1, found = {}",
            profit_usd
        );
    }

    #[test]
    fn cheap_token_test() {
        const EPS: f64 = 0.0001;
        let token_price_usd = 1. / 10_000.;
        let token_amount = web3::types::U256::from(20_000);

        let profit_usd = get_profit_usd(
            token_price_usd,
            token_amount,
            Decimals::try_from(0).unwrap(),
            1.0,
        )
        .unwrap();

        assert!(
            profit_usd - 1.0 < EPS && 1.0 - profit_usd < EPS,
            "Incorrect profit: expected = 1, found = {}",
            profit_usd
        );
    }

    #[test]
    fn format_token_units_zero_test() {
        format_token_units(web3::types::U256::zero(), Decimals::try_from(0).unwrap());
    }

    #[test]
    fn format_token_units_max_decimal_test() {
        format_token_units(web3::types::U256::zero(), Decimals::try_from(24).unwrap());
    }

    #[test]
    fn format_token_units_max_value_zero_decimals_test() {
        format_token_units(web3::types::U256::MAX, Decimals::try_from(0).unwrap());
    }

    #[test]
    fn format_token_units_max_value_and_decimal_test() {
        format_token_units(web3::types::U256::MAX, Decimals::try_from(24).unwrap());
    }

    #[test]
    #[should_panic(expected = "The decimals value is too big. Max value = 24, found = 25")]
    fn format_token_units_min_overflow_decimal_test() {
        format_token_units(web3::types::U256::zero(), Decimals::try_from(25).unwrap());
    }

    #[test]
    fn get_profit_usd_edge_cases() {
        get_profit_usd(
            0.,
            web3::types::U256::zero(),
            Decimals::try_from(0).unwrap(),
            0.,
        )
        .unwrap();
        get_profit_usd(
            0.,
            web3::types::U256::zero(),
            Decimals::try_from(19).unwrap(),
            0.,
        )
        .unwrap();
        get_profit_usd(
            0.,
            web3::types::U256::MAX,
            Decimals::try_from(19).unwrap(),
            0.,
        )
        .unwrap();
        get_profit_usd(
            0.,
            web3::types::U256::MAX,
            Decimals::try_from(0).unwrap(),
            0.,
        )
        .unwrap();
        get_profit_usd(
            f64::MAX,
            web3::types::U256::MAX,
            Decimals::try_from(19).unwrap(),
            0.,
        )
        .unwrap();
        get_profit_usd(
            f64::MAX,
            web3::types::U256::MAX,
            Decimals::try_from(0).unwrap(),
            0.,
        )
        .unwrap();
        get_profit_usd(
            0.,
            web3::types::U256::zero(),
            Decimals::try_from(0).unwrap(),
            f64::MAX,
        )
        .unwrap();
        get_profit_usd(
            0.,
            web3::types::U256::zero(),
            Decimals::try_from(19).unwrap(),
            f64::MAX,
        )
        .unwrap();
        get_profit_usd(
            0.,
            web3::types::U256::MAX,
            Decimals::try_from(19).unwrap(),
            f64::MAX,
        )
        .unwrap();
        get_profit_usd(
            0.,
            web3::types::U256::MAX,
            Decimals::try_from(0).unwrap(),
            f64::MAX,
        )
        .unwrap();
        get_profit_usd(
            f64::MAX,
            web3::types::U256::MAX,
            Decimals::try_from(19).unwrap(),
            f64::MAX,
        )
        .unwrap();
        get_profit_usd(
            f64::MAX,
            web3::types::U256::MAX,
            Decimals::try_from(0).unwrap(),
            f64::MAX,
        )
        .unwrap();
    }
}
