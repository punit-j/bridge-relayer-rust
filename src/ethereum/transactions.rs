use secp256k1::SecretKey;
use web3::types::TransactionId;
use web3::{api, contract::Contract, types::Address};

#[allow(dead_code)]
pub async fn transfer_token<'a, T: web3::Transport>(
    contract: &'a Contract<T>,
    private_key: &'a SecretKey,
    token: Address,
    receiver: Address,
    amount: u64,
    nonce: web3::types::U256,
    unlock_recipient: String,
) -> web3::error::Result<web3::types::H256> {
    contract
        .signed_call(
            "transferTokens",
            (token, receiver, nonce, amount, unlock_recipient),
            Default::default(),
            private_key,
        )
        .await
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionStatus {
    Pengind,
    Failure(web3::types::U64), // block_number
    Sucess(web3::types::U64),  // block_number
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
        return Ok(TransactionStatus::Pengind);
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
            TransactionStatus::Sucess(block_number)
        });
    }

    Err(web3::error::Error::Unreachable)
}

#[cfg(test)]
pub mod tests {
    use crate::ethereum::transactions::{transaction_status, transfer_token, TransactionStatus};
    use eth_client::test_utils::{
        get_eth_erc20_fast_bridge_contract_abi, get_eth_erc20_fast_bridge_proxy_contract_address,
        get_eth_rpc_url, get_eth_token, get_recipient, get_relay_eth_key,
    };
    use rand::Rng;
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

        assert_eq!(tx_status, TransactionStatus::Sucess(U64::from(8180335)));
    }

    #[tokio::test]
    async fn smoke_transfer_token_test() {
        let eth1_endpoint = get_eth_rpc_url().to_string();

        let transport = web3::transports::Http::new(&eth1_endpoint).unwrap();
        let client = web3::api::Eth::new(transport);
        let bridge_proxy_addres = get_eth_erc20_fast_bridge_proxy_contract_address();
        let contract_abi = get_eth_erc20_fast_bridge_contract_abi().await;

        let contract = web3::contract::Contract::from_json(
            client.clone(),
            bridge_proxy_addres,
            contract_abi.as_bytes(),
        )
        .unwrap();

        let priv_key = get_relay_eth_key();
        let nonce = web3::types::U256::from(rand::thread_rng().gen_range(0..1000000000));

        let token_addr = get_eth_token();
        let res = transfer_token(
            &contract,
            &priv_key,
            token_addr,
            get_recipient(),
            159,
            nonce,
            "alice.testnet".to_string(),
        )
        .await
        .unwrap();

        println!("transaction hash = {:?}", res);
    }
}
