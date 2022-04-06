# near-jsonrpc-client

Lower-level API for interfacing with the NEAR Protocol via JSONRPC.

[![crates.io](https://img.shields.io/crates/v/near-jsonrpc-client?label=latest)](https://crates.io/crates/near-jsonrpc-client)
[![Documentation](https://docs.rs/near-jsonrpc-client/badge.svg)](https://docs.rs/near-jsonrpc-client)
[![Dependency Status](https://deps.rs/crate/near-jsonrpc-client/0.3.0/status.svg)](https://deps.rs/crate/near-jsonrpc-client/0.3.0)

## Usage

Each one of the valid JSON RPC methods are defined in the `methods` module.
For instance, to make a `tx` request, you start with the `tx` module
and construct a request using the `methods::tx::RpcTransactionStatusRequest` struct.

```rust
use near_client::{methods, JsonRpcClient};
use near_jsonrpc_primitives::types::transactions::TransactionInfo;

let mainnet_client = JsonRpcClient::connect("https://archival-rpc.mainnet.near.org");

let tx_status_request = methods::tx::RpcTransactionStatusRequest {
    transaction_info: TransactionInfo::TransactionId {
        hash: "9FtHUFBQsZ2MG77K3x3MJ9wjX3UT8zE1TczCrhZEcG8U".parse()?,
        account_id: "miraclx.near".parse()?,
    },
};

// call a method on the server via the connected client
let tx_status = mainnet_client.call(tx_status_request).await?;

println!("{:?}", tx_status);
```

Check out [`the examples folder`](https://github.com/near/near-jsonrpc-client-rs/tree/master/examples) for a comprehensive list of helpful demos. You can run the examples with `cargo`. For example: `cargo run --example view_account`.

For all intents and purposes, the predefined structures in `methods` should suffice, if you find that they
don't or you crave extra flexibility, well, you can opt in to use the generic constructor `methods::any()` with the `any` feature flag.

In this example, we retrieve only the parts from the genesis config response that we care about.

```toml
# in Cargo.toml
near_client = { ..., features = ["any"] }
```

```rust
use serde::Deserialize;
use serde_json::json;

use near_client::{methods, JsonRpcClient};
use near_primitives::serialize::u128_dec_format;
use near_primitives::types::*;

#[derive(Debug, Deserialize)]
struct PartialGenesisConfig {
    protocol_version: ProtocolVersion,
    chain_id: String,
    genesis_height: BlockHeight,
    epoch_length: BlockHeightDelta,
    #[serde(with = "u128_dec_format")]
    min_gas_price: Balance,
    #[serde(with = "u128_dec_format")]
    max_gas_price: Balance,
    #[serde(with = "u128_dec_format")]
    total_supply: Balance,
    validators: Vec<AccountInfo>,
}

impl methods::RpcHandlerResponse for PartialGenesisConfig {}

let mainnet_client = JsonRpcClient::connect("https://rpc.mainnet.near.org");

let genesis_config_request = methods::any::<Result<PartialGenesisConfig, ()>>(
    "EXPERIMENTAL_genesis_config",
    json!(null),
);

let partial_genesis = mainnet_client.call(genesis_config_request).await?;

println!("{:#?}", partial_genesis);
```
