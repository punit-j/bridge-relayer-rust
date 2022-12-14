use near_jsonrpc_client::{methods, JsonRpcClient, JsonRpcClientConnector};
use near_jsonrpc_primitives::types::query::{QueryResponseKind, RpcQueryResponse};
use near_jsonrpc_primitives::types::transactions::TransactionInfo;
use near_primitives::transaction::{Action, FunctionCallAction, Transaction};
use near_primitives::types::{BlockReference, Finality, FunctionArgs};
use near_primitives::views::{FinalExecutionOutcomeView, QueryRequest};
use tokio::time;
use lazy_static::lazy_static;

lazy_static! {
    static ref DEFAULT_CONNECTOR: JsonRpcClientConnector = JsonRpcClient::with(new_near_rpc_client(Some(std::time::Duration::from_secs(30))));
}

fn new_near_rpc_client(timeout: Option<std::time::Duration>) -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::with_capacity(2);
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/json"),
    );

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
    let access_key_query_response = client
        .call(methods::query::RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: near_primitives::views::QueryRequest::ViewAccessKey {
                account_id: signer.account_id.clone(),
                public_key: signer.public_key.clone(),
            },
        })
        .await?;
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
        let received_at = time::Instant::now();
        let delta = (received_at - sent_at).as_secs();
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
