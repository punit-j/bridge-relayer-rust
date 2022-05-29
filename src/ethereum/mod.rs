mod proof;
mod transactions;

use std::borrow::Borrow;
use std::collections::HashMap;
use std::fs;
use std::str::FromStr;
use near_sdk::BlockHeight;
use web3::contract::{Contract, Options};
use web3::ethabi::Uint;
use web3::types::{H256, TransactionReceipt, U256};
use web3::Web3;
use bytes::{BytesMut, BufMut};
use near_primitives::types::TransactionOrReceiptId::Receipt;
use serde_json::json;
use std::process::Command;
use web3::api::Eth;
use web3::ethabi::ParamType::String;
//use base64;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use url::form_urlencoded::Target;
use web3::transports::Http;

#[derive(Debug)]
pub struct Ethereum {
    client: Web3<Http>,
    contract: Contract<Http>,
    key: secp256k1::SecretKey
}

impl Ethereum {
    pub fn new(url: &str,
               contract_addr: web3::ethabi::Address,
               abi_json: &[u8],
               key: secp256k1::SecretKey
    ) -> Result<Self, std::string::String> {
        let transport = web3::transports::Http::new(url).unwrap();
        let client = web3::Web3::new(transport);

        let contract = web3::contract::Contract::from_json(client.eth(), contract_addr, &*abi_json)
            .map_err(|e| e.to_string())?;

        Ok(Self {
            client: client,
            contract: contract,
            key: key
        })
    }

    pub async fn transfer_token(&self, token: web3::ethabi::Address,
                                receiver: web3::ethabi::Address,
                                amount: u64,
                                nonce: web3::types::U256
    ) -> web3::error::Result<web3::types::H256> {
        transactions::transfer_token(&self.contract, &self.key, token, receiver, amount, nonce).await
    }
}

fn get_client() -> web3::Web3<web3::transports::Http> {
    let transport = web3::transports::Http::new("https://goerli.infura.io/v3/05155f003f604cd884bfd577c2219da5").unwrap();
    let client = web3::Web3::new(transport);
    client
}

pub async fn get_proof_nodejs() {
    let client = get_client().eth();

    let res = proof::get_proof(&"https://goerli.infura.io/v3/05155f003f604cd884bfd577c2219da5".to_string(),
                               &client, &"/home/misha/trash/rr/rainbow-bridge/cli/index.js".to_string(),
                               &H256::from_str("0xcb50c668e750650fc53d0027112d0580b42f3b658780598cb6899344e2b94183").unwrap()).await;
    println!("res {:?}", res);
    /*
    let serialized: Vec<u8> = pp.try_to_vec().unwrap();
    //println!("base64 {:?}", base64::encode(serialized));*/
}

pub async fn doit() {
    let abi = fs::read("/home/misha/trash/abi.json").unwrap();
    let priv_key = secp256k1::SecretKey::from_str(&(fs::read_to_string("/home/misha/trash/acc2prk").unwrap().as_str())[..64]).unwrap();
    let contract_addr = web3::types::Address::from_str("5c739e4039D552E2DBF94ce9E7Db261c88BcEc84").unwrap();
    let token_addr = web3::types::Address::from_str("b2d75C5a142A68BDA438e6a318C7FBB2242f9693").unwrap();
    /*
    let client = get_client();

    let my_addr = web3::types::Address::from_str("51599eC779c5fd6b59c5aCc6a31D8e174D8A793E").unwrap();


    //accounts.push(a);
    println!("aaaa {}", my_addr);

    let b = client.eth().balance(my_addr, Option::None).await;
    println!("bal {:?}", b);


    let contract = web3::contract::Contract::from_json(client.eth(), contract_addr, &*abi);

    println!("contr {:?}", contract);
    let contract = contract.unwrap();

    let res = transactions::transfer_token(&contract, &priv_key, token_addr,
                                           web3::types::Address::from_str("2a23E0Fa3Afe77AFf5dc6c6a007E3A10c1450633").unwrap(),
                                           150, web3::types::U256::from(200)).await;
    println!("transferTokens {:?}", res);
*/

    let eth = Ethereum::new("https://goerli.infura.io/v3/05155f003f604cd884bfd577c2219da5",
                            contract_addr,
                            &*abi, priv_key).unwrap();

    let res = eth.transfer_token(token_addr,
                                 web3::types::Address::from_str("2a23E0Fa3Afe77AFf5dc6c6a007E3A10c1450633").unwrap(),
                                 151, web3::types::U256::from(200)).await;
    println!("transferTokens {:?}", res);


    /*let proof = proof::get_proof(&"https://goerli.infura.io/v3/05155f003f604cd884bfd577c2219da5".to_string(),
                     &client.eth(), &"/home/misha/trash/rr/rainbow-bridge/cli/index.js".to_string(),
                     &res.unwrap()).await;
    println!("proof {:?}", proof);*/
}