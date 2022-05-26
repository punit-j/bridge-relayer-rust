extern crate serde;

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
use base64;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

fn get_client() -> web3::Web3<web3::transports::Http> {
    let transport = web3::transports::Http::new("https://goerli.infura.io/v3/05155f003f604cd884bfd577c2219da5").unwrap();
    let client = web3::Web3::new(transport);
    client
}

// Processes case if input == 0 but rlp::encode processes it correct
fn encode(input: &web3::types::Index) -> bytes::BytesMut {
    if *input == web3::types::U64::from(0) {
        return rlp::encode(&Vec::new());
    }
    rlp::encode(input)
}

pub async fn get_proof() {
    let client = get_client().eth();

    let tr_hash = H256::from_str("0x2d2312374f04069d603accfc6a05c80d2ea7f48dccb073cee7ac7800b7da98ee").unwrap();
    println!("tr_hash {}", tr_hash);

    let receipt = client.transaction_receipt(tr_hash).await.unwrap();
    println!("receipt {:?}", receipt);
    let receipt = receipt.unwrap();
    receipt.block_number.unwrap();

    // get log of block contains this transaction
    let logs = client.logs(web3::types::FilterBuilder::default()
        .block_hash(receipt.block_hash.unwrap())
        .address(vec!(receipt.to.unwrap()))// contract address
        .build()).await.unwrap();
    println!("logs {:?}", logs);
    let log = logs.iter().find(|&log| log.transaction_hash.unwrap() == tr_hash);
    println!("log {:?}", log);

    let log_index = log.unwrap().log_index.unwrap();

    let block = client.block(web3::types::BlockId::Hash(receipt.block_hash.unwrap())).await.unwrap();
    println!("block {:?}", block);
    let block = block.unwrap();

    // build trie
    //let mut trie = HashMap::new();
    for transaction in block.transactions {
        let receipt = client.transaction_receipt(transaction).await.unwrap().unwrap();
        let path = rlp::encode(&receipt.transaction_index);
        //let serialized_receipt = receiptFromWeb3(receipt).serialize();
        //println!("path {:?} {}", path.to_vec(), receipt.transaction_index);

        let path = encode(&receipt.transaction_index);

        let mut receipt_json = serde_json::to_value(&receipt).unwrap();//.as_object_mut().unwrap();

        //let tt = serde_json::json!(format!("{:X}", receipt.cumulative_gas_used));

        *receipt_json.get_mut("cumulativeGasUsed").unwrap() = serde_json::json!(format!("{:X}", receipt.cumulative_gas_used));

        let mut status_j = receipt_json.get_mut("status").unwrap();
        if let Some(s) = receipt.status {
            *status_j = serde_json::Value::String((if s != web3::types::U64::from(0) {"0x1"} else {"0x0"}).parse().unwrap());
        }

        /*receipt_json.cumulativeGasUsed = web3.utils.toHex(rpcResult.cumulativeGasUsed)
        if (web3Result.status === true) {
            rpcResult.status = '0x1'
        } else if (web3Result.status === false) {
            rpcResult.status = '0x0'
        }*/

        println!("receipt_json {:?}", receipt_json);
    }
}

pub async fn get_transaction_log_index(client: &web3::api::Eth<web3::transports::Http>, tr_hash: H256) -> Option<U256> {
    let receipt = client.transaction_receipt(tr_hash).await.unwrap();

    let receipt = receipt.unwrap();
    receipt.block_number.unwrap();
    // get log of block contains this transaction
    let logs = client.logs(web3::types::FilterBuilder::default()
        .block_hash(receipt.block_hash.unwrap())
        .address(vec!(receipt.to.unwrap()))// contract address
        .build()).await.unwrap();

    let log = logs.iter().find(|&log| log.transaction_hash.unwrap() == tr_hash);

    log.unwrap().log_index
}

#[derive(Default, BorshDeserialize, BorshSerialize, Clone, Serialize, Deserialize)]
pub struct Proof {
    pub log_index: u64,
    pub log_entry_data: Vec<u8>,
    pub receipt_index: u64,
    pub receipt_data: Vec<u8>,
    pub header_data: Vec<u8>,
    pub proof: Vec<Vec<u8>>,
    pub skip_bridge_call: bool,
}

pub async fn get_proof_nodejs() {
    let client = get_client().eth();

    let tr_hash = H256::from_str("0xcb50c668e750650fc53d0027112d0580b42f3b658780598cb6899344e2b94183").unwrap();
    println!("tr_hash {:#x}", tr_hash);

    let log_index = get_transaction_log_index(&client, tr_hash).await.unwrap();
    println!("log_index {}", log_index);
    let json_args = json!({"logIndex": log_index.as_u64(), "transactionHash": tr_hash});

    let mut tt = Command::new("node");
    tt.arg("/home/misha/trash/rr/rainbow-bridge/cli/index.js").arg("eth-to-near-find-proof")
        .arg(json_args.to_string())
        .arg("--eth-node-url").arg("https://goerli.infura.io/v3/05155f003f604cd884bfd577c2219da5");
    println!("wbb {:?}", tt.get_args());
    let rr = tt.output().unwrap().stdout;
    let mut out = std::str::from_utf8(&rr).unwrap();
    //let out = std::string::String::from_utf8(rr);
    //let out = out.;

    let mut j = serde_json::from_str::<serde_json::Value>(out).unwrap();
    let mut proof_locker = j.get_mut("proof_locker").unwrap().take();
    proof_locker.as_object_mut().unwrap().insert("skip_bridge_call".parse().unwrap(), serde_json::Value::Bool(false));

    println!("json {:?}", proof_locker.to_string());

    let pp = serde_json::from_value::<Proof>(proof_locker).unwrap();

    let serialized: Vec<u8> = pp.try_to_vec().unwrap();
    println!("base64 {:?}", base64::encode(serialized));
}

pub async fn doit() {
    let client = get_client();

    println!("Calling accounts.");
    let mut accounts = client.eth().accounts().await.unwrap();
    println!("Accounts: {:?}", accounts);

    let my_addr = web3::types::Address::from_str("51599eC779c5fd6b59c5aCc6a31D8e174D8A793E").unwrap();
    let priv_key = secp256k1::SecretKey::from_str(&(fs::read_to_string("/home/misha/trash/acc2prk").unwrap().as_str())[..64]).unwrap();

    let contract_addr = web3::types::Address::from_str("5c739e4039D552E2DBF94ce9E7Db261c88BcEc84").unwrap();
    let token_addr = web3::types::Address::from_str("b2d75C5a142A68BDA438e6a318C7FBB2242f9693").unwrap();
    //accounts.push(a);
    println!("aaaa {}", my_addr);

    let b = client.eth().balance(my_addr, Option::None).await;
    println!("bal {:?}", b);

    let abi = fs::read("/home/misha/trash/abi.json").unwrap();
    let contract = web3::contract::Contract::from_json(client.eth(), contract_addr, &*abi);

    println!("contr {:?}", contract);
    let contract = contract.unwrap();

    let res: web3::types::Address = contract.query("owner", (), None, Default::default(), None).await.unwrap();
    println!("owner {:?}", res);

    let res: bool = contract.query("isTokenInWhitelist", (token_addr, ),
                                   None, Default::default(), None).await.unwrap();
    println!("isTokenInWhitelist {:?}", res);

    let res = contract.signed_call("transferTokens", (token_addr,
                                                      web3::types::Address::from_str("2a23E0Fa3Afe77AFf5dc6c6a007E3A10c1450633").unwrap(),  // to
                                                      Uint::from(112),
                                                      Uint::from(10)),    // amount
                                   Default::default(),
                                   &priv_key).await;
    println!("transferTokens {:?}", res);

    //contract.call("transferTokens", (), my_addr, Default::default())
}