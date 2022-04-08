# near_client

Lower-level API for interfacing with the NEAR Protocol via JSONRPC.

[![crates.io](https://img.shields.io/crates/v/near-jsonrpc-client?label=latest)](https://crates.io/crates/near-jsonrpc-client)
[![Documentation](https://docs.rs/near-jsonrpc-client/badge.svg)](https://docs.rs/near-jsonrpc-client)
[![Dependency Status](https://deps.rs/crate/near-jsonrpc-client/0.3.0/status.svg)](https://deps.rs/crate/near-jsonrpc-client/0.3.0)

Check out [`the examples folder`](https://github.com/near/near-jsonrpc-client-rs/tree/master/examples) for a comprehensive list of helpful demos.

## Example of calling view method

TO RUN the example, create an "examples" folder in the "near-client" folder, then create a "contract_view_method.rs" file and put the code below in it, changing the parameter values to your own, then write in the CLI "cargo run --example contract_view_method"

```rust

use near_jsonrpc_primitives::types::query::QueryResponseKind;
use serde_json::{from_slice, json};

use near_client::methods::view;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let response = view(
        "https://rpc.testnet.near.org".to_string(), // server_addr
        "arseniyrest.testnet".to_string(), // contract_account_id
        "get_num".to_string(), // method_name
        json!({}), // args
    )
    .await;
    if let QueryResponseKind::CallResult(result) = response.unwrap().kind {
        println!("{:#?}", from_slice::<i8>(&result.result)?); // i8 — type of the result returned by the get_num method
    }
    Ok(())
}

```

## Example of calling change method

TO RUN the example, create an "examples" folder in the "near-client" folder, then create a "contract_change_method.rs" file and put the code below in it, changing the parameter values to your own, then write in the CLI "cargo run --example contract_change_method"

```rust

use near_client::methods::change;
use near_client::read_private_key;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let response = change(
        "https://rpc.testnet.near.org".to_string(), // server_addr
        "arseniyrest.testnet".to_string(), // signer_account_id
        read_private_key::read_private_key_from_file(
            "/home/arseniyk/.near-credentials/testnet/arseniyrest.testnet.json",
        ), // signer_secret_key
        "arseniyrest.testnet".to_string(), // receiver_id
        "increment".to_string(), // method_name
        json!({}), // args
        100_000_000_000_000, // gas
        0, // deposit
    )
    .await;
    println!("{:#?}", response.unwrap());
    Ok(())
}

```
