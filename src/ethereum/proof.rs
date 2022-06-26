//! Generation proof for hash using the rainbow-bridge
//!
//! To use this module you need to clone the https://github.com/aurora-is-near/rainbow-bridge (commit b759c1bef868b8609c15c72e35d6647985a97315),
//! install the NodeJS and install node_modules.
//! To check you can to cal ./cli/index.js
//!
//! # Example
//!
//! ```
//! let url = "https://goerli.infura.io/v3/<your api key>";
//! let transport = web3::transports::Http::new(url).unwrap();
//! let client = web3::Web3::new(transport);
//!
//! let transaction_hash = H256::from_str("0xcb50c668e750650fc53d0027112d0580b42f3b658780598cb6899344e2b94183").unwrap();
//!
//! let res = proof::get_proof(&url.to_string(), &client,
//!                             &"rainbow-bridge/cli/index.js".to_string(),
//!                             &transaction_hash)
//! .await;
//!
//! println!("res {:?}", res);
//! ```

use serde_json::json;
use spectre_bridge_common;
use std::{process};
use web3::{
    api,
    types::{H256, U256},
};

#[derive(Debug)]
pub enum Error<'a> {
    Empty,
    Other(&'a str),
    Web3(web3::Error),
    Json(serde_json::Error),
}

pub async fn get_proof<'a, 'b, T: web3::Transport>(
    url: &'a str,
    client: &'a api::Eth<T>,
    rb_bridge_index_js_url: &'a str,
    tx_hash: &'a H256,
) -> Result<spectre_bridge_common::Proof, Error<'b>> {
    let log_index = get_transaction_log_index(client, tx_hash).await?;

    let json_args = json!({"logIndex": log_index.as_u64(), "transactionHash": tx_hash});

    let mut command = process::Command::new("node");
    command
        .arg(rb_bridge_index_js_url)
        .arg("eth-to-near-find-proof")
        .arg(json_args.to_string())
        .arg("--eth-node-url")
        .arg(url);

    let rr = command
        .output()
        .map_err(|_e| Error::Other("Unable to unwrap output"))?
        .stdout;
    let out = std::str::from_utf8(&rr).map_err(|_e| Error::Other("Unable to parse output"))?;

    let json = serde_json::from_str::<serde_json::Value>(out).map_err(Error::Json)?;
    let json = json
        .get("proof_locker")
        .ok_or(Error::Other("JSON doesnt contain the proof_locker"))?;

    let res = serde_json::from_value::<spectre_bridge_common::Proof>(json.clone())
        .map_err(Error::Json)?;
    Ok(res)
}

pub async fn get_transaction_log_index<'a, 'b, T: web3::Transport>(
    client: &'a api::Eth<T>,
    tx_hash: &'a H256,
) -> Result<U256, Error<'b>> {
    let receipt = client
        .transaction_receipt(*tx_hash)
        .await
        .map_err(Error::Web3)?
        .ok_or(Error::Other("Unable to unwrap receipt"))?;

    // get log of block contains this transaction
    let logs = client
        .logs(
            web3::types::FilterBuilder::default()
                .block_hash(
                    receipt
                        .block_hash
                        .ok_or(Error::Other("Unable to unwrap the 'block_hash'"))?,
                )
                .address(vec![receipt
                    .to
                    .ok_or(Error::Other("Unable to unwrap the 'to'"))?]) // contract address
                .build(),
        )
        .await
        .map_err(Error::Web3)?;

    let log = logs
        .iter()
        .find(|&log| {
            if let Some(hash) = log.transaction_hash {
                if hash == *tx_hash {
                    return true;
                }
            };
            false
        })
        .ok_or(Error::Other("Log not found"))?;

    log.log_index.ok_or(Error::Empty)
}
