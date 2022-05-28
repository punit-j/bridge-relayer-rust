
use std::process;
use serde_json::json;
use web3::contract::{Contract, Options};
use web3::ethabi::ParamType::String;
use web3::ethabi::Uint;
use web3::types::{H256, TransactionReceipt, U256};
use web3::Web3;
use spectre_bridge_common;
use web3::api::Eth;

#[derive(Debug)]
pub enum Error<'a> {
    Empty,
    Other(&'a str),
    Web3(web3::Error),
    Json(serde_json::Error),
}

pub async fn get_proof<T: web3::Transport>(url: std::string::String,
                                           client: &web3::api::Eth<T>,
                                           index_js: std::string::String,
                                           tr_hash: H256)
    -> Result<spectre_bridge_common::Proof, Error<'_>> {
    let log_index = get_transaction_log_index(&client, tr_hash).await?;

    let json_args = json!({"logIndex": log_index.as_u64(), "transactionHash": tr_hash});

    let mut command = process::Command::new("node");
    command.arg(index_js).arg("eth-to-near-find-proof")
        .arg(json_args.to_string())
        .arg("--eth-node-url").arg(url);

    let rr = command.output().map_err(|e| Error::Other("Unable to unwrap output"))?.stdout;
    let mut out = std::str::from_utf8(&rr).map_err(|e| Error::Other("Unable to parse output"))?;

    let json = serde_json::from_str::<serde_json::Value>(out).map_err(|e| Error::Json(e))?;
    let json= json.get("proof_locker").ok_or(Error::Other("JSON doesnt contain the proof_locker"))?;

    let res = serde_json::from_value::<spectre_bridge_common::Proof>(json.clone()).map_err(|e| Error::Json(e))?;
    Ok(res)
}

pub async fn get_transaction_log_index<T: web3::Transport>(client: &web3::api::Eth<T>, tr_hash: H256) -> Result<U256, Error<'_>> {
    let receipt = client.transaction_receipt(tr_hash)
        .await
        .map_err(|e| Error::Web3(e))?
        .ok_or(Error::Other("Unable to unwrap receipt"))?;

    // get log of block contains this transaction
    let logs = client.logs(web3::types::FilterBuilder::default()
        .block_hash(receipt.block_hash.ok_or(Error::Other("Unable to unwrap the 'block_hash'"))?)
        .address(vec!(receipt.to.ok_or(Error::Other("Unable to unwrap the 'to'"))?))// contract address
        .build())
        .await
        .map_err(|e| Error::Web3(e))?;

    let log = logs.iter()
        .find(|&log| {
            if let Some(hash) = log.transaction_hash {
                if hash == tr_hash {
                    return true;
                }
            };
            false
        }).ok_or(Error::Other("Log not found"))?;

    log.log_index.ok_or(Error::Empty)
}
