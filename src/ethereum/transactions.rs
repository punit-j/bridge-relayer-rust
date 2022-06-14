use std::string;
use secp256k1::SecretKey;
use web3::{
    contract::Contract,
    types::Address,
    api,
    ethabi
};
use web3::types::TransactionId;

pub async fn transfer_token<'a, T: web3::Transport>(contract: &'a Contract<T>,
                                                    private_key: &'a SecretKey,
                                                    token: Address,
                                                    receiver: Address,
                                                    amount: u64,
                                                    nonce: web3::types::U256)
    -> web3::error::Result<web3::types::H256> {
    let res = contract.signed_call("transferTokens",
                                   (token, receiver, nonce, amount),
                                   Default::default(), private_key)
        .await;
    res
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionStatus {
    Pengind,
    Failure(web3::types::U64),  // block_number
    Sucess(web3::types::U64)  // block_number
}

pub async fn transaction_status<T: web3::Transport>(client: &api::Eth<T>, tx_hash: web3::types::H256)
                                                    -> web3::error::Result<TransactionStatus> {
    let res = client.transaction(TransactionId::from(tx_hash.clone())).await?.ok_or(web3::error::Error::Unreachable)?;
    if res.block_number.is_none() {
        return Ok(TransactionStatus::Pengind);
    }

    let res = client.transaction_receipt(tx_hash).await?.ok_or(web3::error::Error::Unreachable)?;
    if let Some(s) = res.status {
        let block_number = res.block_number.unwrap();
        return Ok(if s==web3::types::U64::from(0) {TransactionStatus::Failure(block_number) } else {TransactionStatus::Sucess(block_number)});
    }

    Err(web3::error::Error::Unreachable)
}

