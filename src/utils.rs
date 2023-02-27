use crate::{async_redis_wrapper::AsyncRedisWrapper, errors::CustomError};
use url::Url;

pub async fn request_interval(seconds: u64) -> tokio::time::Interval {
    tokio::time::interval_at(
        tokio::time::Instant::now() + tokio::time::Duration::from_secs(seconds),
        tokio::time::Duration::from_secs(seconds),
    )
}

pub async fn get_tx_count(
    redis: &mut AsyncRedisWrapper,
    rpc_url: Url,
    relay_eth_address: web3::types::Address,
) -> Result<web3::types::U256, CustomError> {
    let transaction_count = redis
        .get_transaction_count()
        .await
        .unwrap_or(Some(0.into()))
        .unwrap_or(0.into());

    let transaction_count_rpc =
        eth_client::methods::get_transaction_count(rpc_url.as_str(), relay_eth_address)
            .await
            .map_err(|e| crate::errors::CustomError::FailedGetTxCount(e))?;

    Ok(std::cmp::max(transaction_count, transaction_count_rpc))
}
