use near_client::{methods, JsonRpcClient};
use near_jsonrpc_primitives::types::query::QueryResponseKind;
use near_primitives::types::{BlockReference, Finality, FunctionArgs};
use near_primitives::views::QueryRequest;

use serde_json::{from_slice, json};

mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = JsonRpcClient::connect("https://rpc.testnet.near.org");

    let request = methods::query::RpcQueryRequest {
        block_reference: BlockReference::Finality(Finality::Final),
        request: QueryRequest::CallFunction {
            account_id: "arseniyrest.testnet".parse()?,
            method_name: "get_num".to_string(),
            args: FunctionArgs::from(json!({}).to_string().into_bytes()),
        },
    };

    let response = client.call(request).await?;

    if let QueryResponseKind::CallResult(result) = response.kind {
        println!("{:#?}", from_slice::<i8>(&result.result)?);
    }

    Ok(())
}
