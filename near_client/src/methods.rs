use lazy_static::lazy_static;
use near_jsonrpc_client::{methods, JsonRpcClient, JsonRpcClientConnector};
use near_jsonrpc_primitives::types::query::{QueryResponseKind, RpcQueryResponse};
use near_jsonrpc_primitives::types::transactions::TransactionInfo;
use near_primitives::transaction::{Action, FunctionCallAction, Transaction};
use near_primitives::types::{BlockReference, Finality, FunctionArgs};
use near_primitives::views::{FinalExecutionOutcomeView, QueryRequest};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use tokio::time;

lazy_static! {
    static ref DEFAULT_CONNECTOR: JsonRpcClientConnector = JsonRpcClient::with(
        new_near_rpc_client(Some(std::time::Duration::from_secs(30)))
    );
}

fn new_near_rpc_client(timeout: Option<std::time::Duration>) -> reqwest::Client {
    let mut headers = HeaderMap::with_capacity(2);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let mut builder = reqwest::Client::builder().default_headers(headers);
    if let Some(timeout) = timeout {
        builder = builder.timeout(timeout).connect_timeout(timeout);
    }
    builder.build().unwrap()
}

pub async fn view(
    server_addr: url::Url,
    contract_account_id: String,
    method_name: String,
    args: serde_json::Value,
) -> Result<RpcQueryResponse, Box<dyn std::error::Error>> {
    let client = DEFAULT_CONNECTOR.connect(server_addr);
    let request = methods::query::RpcQueryRequest {
        block_reference: BlockReference::Finality(Finality::Final),
        request: QueryRequest::CallFunction {
            account_id: contract_account_id.parse()?,
            method_name,
            args: FunctionArgs::from(args.to_string().into_bytes()),
        },
    };
    Ok(client.call(request).await?)
}

pub async fn get_final_block_timestamp(
    server_addr: url::Url,
) -> Result<u64, Box<dyn std::error::Error>> {
    let client = DEFAULT_CONNECTOR.connect(server_addr);
    let request = methods::block::RpcBlockRequest {
        block_reference: BlockReference::Finality(Finality::Final),
    };

    let block_info = client.call(request).await?;
    Ok(block_info.header.timestamp)
}

pub async fn get_last_near_block_height(
    server_addr: url::Url,
) -> Result<u64, Box<dyn std::error::Error>> {
    let client = DEFAULT_CONNECTOR.connect(server_addr);
    let request = methods::block::RpcBlockRequest {
        block_reference: BlockReference::latest(),
    };

    let block_info = client.call(request).await?;
    Ok(block_info.header.height as u64)
}

pub async fn change(
    server_addr: url::Url,
    signer: near_crypto::InMemorySigner,
    receiver_id: String,
    method_name: String,
    args: serde_json::Value,
    gas: u64,
    deposit: u128,
) -> Result<FinalExecutionOutcomeView, Box<dyn std::error::Error>> {
    let client = DEFAULT_CONNECTOR.connect(server_addr);
    let rpc_request = methods::query::RpcQueryRequest {
        block_reference: BlockReference::latest(),
        request: near_primitives::views::QueryRequest::ViewAccessKey {
            account_id: signer.account_id.clone(),
            public_key: signer.public_key.clone(),
        },
    };
    let access_key_query_response = client.call(rpc_request).await?;
    let current_nonce = match access_key_query_response.kind {
        QueryResponseKind::AccessKey(access_key) => access_key.nonce,
        _ => Err("failed to extract current nonce")?,
    };
    let transaction = Transaction {
        signer_id: signer.account_id.clone(),
        public_key: signer.public_key.clone(),
        nonce: current_nonce + 1,
        receiver_id: receiver_id.parse()?,
        block_hash: access_key_query_response.block_hash,
        actions: vec![Action::FunctionCall(FunctionCallAction {
            method_name,
            args: args.to_string().into_bytes(),
            gas,
            deposit,
        })],
    };
    let request = methods::broadcast_tx_async::RpcBroadcastTxAsyncRequest {
        signed_transaction: transaction.sign(&signer),
    };
    let sent_at = time::Instant::now();
    let tx_hash = client.call(request).await?;
    loop {
        let response = client
            .call(methods::tx::RpcTransactionStatusRequest {
                transaction_info: TransactionInfo::TransactionId {
                    hash: tx_hash,
                    account_id: signer.account_id.clone(),
                },
            })
            .await;

        let delta = (time::Instant::now() - sent_at).as_secs();
        if delta > 60 {
            Err("time limit exceeded for the transaction to be recognized")?;
        }

        match response {
            Err(err) => match err.handler_error() {
                Some(err) => {
                    eprintln!("(An handler error occurred `{:#?}`", err);
                    time::sleep(time::Duration::from_secs(2)).await;
                    continue;
                }
                _ => Err(err)?,
            },
            Ok(response) => return Ok(response),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::methods::{change, view};
    use crate::test_utils::{get_near_signer, get_near_token, get_server_addr};
    use near_primitives::views::FinalExecutionStatus;
    use near_sdk::borsh::BorshDeserialize;
    use serde_json::json;
    use std::time::SystemTime;

    #[tokio::test]
    async fn smoke_blocktimestamp_test() {
        const MIN_IN_NS: u64 = 60_000_000_000;

        let near_timestamp_ns = crate::methods::get_final_block_timestamp(get_server_addr())
            .await
            .unwrap();

        let sys_timestamp_ns = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        println!("near timestamp: {:?}", near_timestamp_ns);
        println!("sys timestamp: {:?}", sys_timestamp_ns);

        assert!(near_timestamp_ns < sys_timestamp_ns + MIN_IN_NS);
        assert!(sys_timestamp_ns < near_timestamp_ns + MIN_IN_NS);
    }

    #[tokio::test]
    async fn smoke_view_test() {
        let server_addr = get_server_addr();
        let contract_account_id = "client6.goerli.testnet".to_string();
        let method_name = "last_block_number".to_string();
        let args = json!({});

        let response = view(server_addr, contract_account_id, method_name, args)
            .await
            .unwrap();

        if let near_jsonrpc_primitives::types::query::QueryResponseKind::CallResult(result) =
            response.kind
        {
            let value = u64::try_from_slice(&result.result).unwrap();

            println!("last block number = {}", value);
        } else {
            panic!("Error on unwraping view result")
        }
    }

    #[tokio::test]
    async fn smoke_change_test() {
        let server_addr = get_server_addr();
        let contract_account_id = get_near_token();
        let method_name = "mint".to_string();
        let signer = get_near_signer();

        let args = json!({"account_id": signer.account_id, "amount": "100"});

        let response = change(
            server_addr,
            signer,
            contract_account_id.to_string(),
            method_name,
            args,
            4_000_000_000_000,
            0,
        )
        .await
        .unwrap();

        if let FinalExecutionStatus::SuccessValue(_) = response.status {
            println!("change response = {:?}", response);
        } else {
            panic!("Response status not success: {:?}", response)
        }
    }
}
