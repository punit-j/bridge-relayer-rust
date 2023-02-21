use web3::api;
use web3::types::TransactionId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionStatus {
    Pending,
    Failure(web3::types::U64), // block_number
    Success(web3::types::U64), // block_number
}

pub async fn transaction_status<T: web3::Transport>(
    client: &api::Eth<T>,
    tx_hash: web3::types::H256,
) -> web3::error::Result<TransactionStatus> {
    let res = client
        .transaction(TransactionId::from(tx_hash))
        .await?
        .ok_or(web3::error::Error::Unreachable)?;
    if res.block_number.is_none() {
        return Ok(TransactionStatus::Pending);
    }

    let res = client
        .transaction_receipt(tx_hash)
        .await?
        .ok_or(web3::error::Error::Unreachable)?;
    if let Some(s) = res.status {
        let block_number = res.block_number.unwrap();
        return Ok(if s == web3::types::U64::from(0) {
            TransactionStatus::Failure(block_number)
        } else {
            TransactionStatus::Success(block_number)
        });
    }

    Err(web3::error::Error::Unreachable)
}

#[cfg(test)]
pub mod tests {
    use crate::ethereum::transactions::{transaction_status, TransactionStatus};
    use eth_client::test_utils::get_eth_rpc_url;
    use web3::api::Namespace;
    use web3::types::U64;

    #[tokio::test]
    async fn smoke_transaction_status_test() {
        let tx_hash = web3::types::H256::from_slice(
            &hex::decode("564e7a804e74e45710021c692a0fdc2ef5284bc4fbfd3b552b359adb89e21f14")
                .unwrap(),
        );

        let eth1_endpoint = get_eth_rpc_url().to_string();

        let transport = web3::transports::Http::new(&eth1_endpoint).unwrap();
        let client = web3::api::Eth::new(transport);

        let tx_status = transaction_status(&client, tx_hash).await.unwrap();

        assert_eq!(tx_status, TransactionStatus::Success(U64::from(8180335)));
    }
}
