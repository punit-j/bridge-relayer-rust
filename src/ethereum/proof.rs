use web3::contract::{Contract, Options};
use web3::ethabi::ParamType::String;
use web3::ethabi::Uint;
use web3::types::{H256, TransactionReceipt, U256};
use web3::Web3;

#[derive(Debug)]
pub enum Error<'a> {
    Empty,
    Other(&'a str),
    Web3(web3::Error)
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
