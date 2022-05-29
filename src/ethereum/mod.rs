mod proof;
mod transactions;

use core::time;
use std::{
    borrow::Borrow,
    string,
    collections::HashMap,
    fs,
    str::FromStr,
    process::Command,
    thread::sleep
};
use near_sdk::{
    BlockHeight,
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize}
};
use web3::{
    contract::Contract,
    ethabi,
    types,
    transports::Http,
    api::Namespace
};
use bytes::{BytesMut, BufMut};
use near_primitives::types::TransactionOrReceiptId::Receipt;
use serde_json::json;

#[derive(Debug)]
pub struct Ethereum {
    api_url: string::String,
    rainbow_bridge_index: string::String,
    client: web3::api::Eth<Http>,
    contract: Contract<Http>,
    key: secp256k1::SecretKey
}

impl Ethereum {
    pub fn new(url: &str, rainbow_bridge_index: string::String,
               contract_addr: web3::ethabi::Address,
               abi_json: &[u8],
               key: secp256k1::SecretKey
    ) -> Result<Self, std::string::String> {
        let transport = web3::transports::Http::new(url).unwrap();
        let client = web3::api::Eth::new(transport);

        let contract = web3::contract::Contract::from_json(client.clone(), contract_addr, &*abi_json)
            .map_err(|e| e.to_string())?;

        Ok(Self {
            api_url: url.to_string(),
            rainbow_bridge_index,
            client,
            contract,
            key
        })
    }

    pub async fn transfer_token(&self, token: web3::ethabi::Address,
                                receiver: web3::ethabi::Address,
                                amount: u64,
                                nonce: web3::types::U256
    ) -> web3::error::Result<web3::types::H256> {
        transactions::transfer_token(&self.contract, &self.key, token, receiver, amount, nonce).await
    }

    pub async fn get_proof<'a, 'b>(&self, tr_hash: &'a web3::types::H256) -> Result<spectre_bridge_common::Proof, proof::Error<'b>> {
        proof::get_proof(&self.api_url, &self.client, &self.rainbow_bridge_index, &tr_hash).await
    }
}

pub async fn doit() {
    let abi = fs::read("/home/misha/trash/abi.json").unwrap();
    let priv_key = secp256k1::SecretKey::from_str(&(fs::read_to_string("/home/misha/trash/acc2prk").unwrap().as_str())[..64]).unwrap();
    let contract_addr = web3::types::Address::from_str("5c739e4039D552E2DBF94ce9E7Db261c88BcEc84").unwrap();
    let token_addr = web3::types::Address::from_str("b2d75C5a142A68BDA438e6a318C7FBB2242f9693").unwrap();

    let eth = Ethereum::new("https://goerli.infura.io/v3/05155f003f604cd884bfd577c2219da5",
                            "/home/misha/trash/rr/rainbow-bridge/cli/index.js".to_string(),
                            contract_addr,
                            &*abi, priv_key).unwrap();

    let res = eth.transfer_token(token_addr,
                                 web3::types::Address::from_str("2a23E0Fa3Afe77AFf5dc6c6a007E3A10c1450633").unwrap(),
                                 152, web3::types::U256::from(200)).await;
    println!("transferTokens {:?}", res);

    sleep(time::Duration::from_secs(20));
    let res = eth.get_proof(&res.unwrap()).await;
    println!("proof {:?}", res);
}