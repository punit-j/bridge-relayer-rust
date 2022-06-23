//! Operations with eth contact
//!
//! # Example
//!
//! ```
//! let abi = fs::read("/home/misha/trash/abi.json").unwrap();
//! let priv_key = secp256k1::SecretKey::from_str(&(fs::read_to_string("/home/misha/trash/acc2prk").unwrap().as_str())[..64]).unwrap();
//! let contract_addr = web3::types::Address::from_str("5c739e4039D552E2DBF94ce9E7Db261c88BcEc84").unwrap();
//! let token_addr = web3::types::Address::from_str("b2d75C5a142A68BDA438e6a318C7FBB2242f9693").unwrap();
//!
//! let eth = RainbowBridgeEthereumClient::new("https://goerli.infura.io/v3/05155f003f604cd884bfd577c2219da5",
//!                                            "/home/misha/trash/rr/rainbow-bridge/cli/index.js",
//!                                            contract_addr,
//!                                            &*abi, priv_key).unwrap();
//!
//! let res = eth.transfer_token(token_addr,
//!                              web3::types::Address::from_str("2a23E0Fa3Afe77AFf5dc6c6a007E3A10c1450633").unwrap(),
//!                              159, web3::types::U256::from(200)).await;
//! println!("transfer_token hash {:?}", &res);
//! let tx_hash = res.unwrap();
//!
//! // wait for transaction process
//! let res = loop {
//!     sleep(time::Duration::from_secs(2));
//!     let res = eth.transaction_status(tx_hash.clone()).await.unwrap();
//!     if res == transactions::TransactionStatus::Pengind {
//!         continue;
//!     }
//!
//!     break res;
//! };
//!
//! // get proof
//! if res == transactions::TransactionStatus::Sucess {
//!     let proof = eth.get_proof(&tx_hash).await;
//!     println!("proof {:?}", proof);
//! }
//! else {
//!     println!("Transaction is failure");
//! }
//! ```

pub mod proof;
pub mod transactions;

use bytes::{BufMut, BytesMut};
use core::time;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    BlockHeight,
};
use std::time::Duration;
use std::{
    borrow::Borrow, collections::HashMap, fs, process::Command, str::FromStr, string, thread::sleep,
};
use web3::{api::Namespace, contract::Contract, ethabi, transports::Http, types};

#[derive(Debug)]
pub struct RainbowBridgeEthereumClient<'a> {
    api_url: &'a str,
    rainbow_bridge_index: &'a str,
    client: web3::api::Eth<Http>,
    contract: Contract<Http>,
    key: secp256k1::SecretKey,
}

impl<'a> RainbowBridgeEthereumClient<'a> {
    pub fn new(
        url: &'a str,
        rainbow_bridge_index: &'a str,
        contract_addr: web3::ethabi::Address,
        abi_json: &[u8],
        key: secp256k1::SecretKey,
    ) -> Result<Self, std::string::String> {
        let transport = web3::transports::Http::new(url).unwrap();
        let client = web3::api::Eth::new(transport);

        let contract =
            web3::contract::Contract::from_json(client.clone(), contract_addr, &*abi_json)
                .map_err(|e| e.to_string())?;

        Ok(Self {
            api_url: url,
            rainbow_bridge_index,
            client,
            contract,
            key,
        })
    }

    pub async fn transfer_token(
        &self,
        token: web3::ethabi::Address,
        receiver: web3::ethabi::Address,
        amount: u64,
        nonce: web3::types::U256,
    ) -> web3::error::Result<web3::types::H256> {
        transactions::transfer_token(&self.contract, &self.key, token, receiver, amount, nonce)
            .await
    }

    pub async fn transaction_status(
        &self,
        tx_hash: web3::types::H256,
    ) -> web3::error::Result<transactions::TransactionStatus> {
        transactions::transaction_status(&self.client, tx_hash).await
    }

    pub async fn get_proof<'b, 'c>(
        &self,
        tx_hash: &'b web3::types::H256,
    ) -> Result<spectre_bridge_common::Proof, proof::Error<'c>> {
        proof::get_proof(
            self.api_url,
            &self.client,
            self.rainbow_bridge_index,
            &tx_hash,
        )
        .await
    }
}
