use std::string;
use secp256k1::SecretKey;
use web3::{
    contract::Contract,
    types::Address,
    api,
    ethabi
};

pub async fn transfer_token<'a, T: web3::Transport>(contract: &'a Contract<T>,
                                                    private_key: &'a SecretKey,
                                                    token: Address,
                                                    receiver: Address,
                                                    amount: u64,
                                                    nonce: web3::types::U256
) -> web3::error::Result<web3::types::H256> {
    let res = contract.signed_call("transferTokens",
                                   (token, receiver, nonce, amount),
                                   Default::default(), private_key)
        .await;
    res
}