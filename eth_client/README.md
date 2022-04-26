# eth_client

Ethereum JSON-RPC multi-transport client

[docs-rs-url]: https://docs.rs/web3

TO RUN the example, create an "examples" folder in the "eth_client" folder, then create a "contract_view_method.rs" file and put the code below in it, changing the parameter values to your own, then write in the CLI "cargo run --example contract_view_method"

For more see [examples folder](./examples).

## Example of calling view method

```rust

use eth_client::methods::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let view_response: web3::types::U256 = construct_contract_interface(
        "https://rinkeby.infura.io/v3/168bdff2f03e417eb8e69cd90fc54615", // endpoint_url
        "0x8d9Eda359157594F352dc29c0bDB741bb8F6b65e", // contract_addr
        // include_bytes!("PATH/file.abi"), // or contract_abi (from file) (leave this line or)
        get_contract_abi( // or contract_abi (by contract_addr) (leave this line)
        "https://api-rinkeby.etherscan.io", // etherscan endpoint_url
        "0x8d9Eda359157594F352dc29c0bDB741bb8F6b65e", // contract_addr
        "" // etherscan your api token (not necessary)
    )
    .await?
    .as_bytes(),
    )?
    .query(
        "retrieve", // // method_name
        (), // args
        None, // default
        web3::contract::Options::default(), // default
        None, // default
    )
    .await?;
    println!("value: {}", view_response);
    Ok(())
}

```

TO RUN the example, create an "examples" folder in the "eth_client" folder, then create a "contract_change_method.rs" file and put the code below in it, changing the parameter values to your own, then write in the CLI "cargo run --example contract_change_method"

## Example of calling change method

```rust

use eth_client::methods::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let change_response = change("https://rinkeby.infura.io/v3/168bdff2f03e417eb8e69cd90fc54615", // infura endpoint_url
    "0x8d9Eda359157594F352dc29c0bDB741bb8F6b65e", // contract_addr
    // include_bytes!("PATH/file.abi"), // or contract_abi (from file) (leave this line or)
    get_contract_abi( // or contract_abi (by contract_addr)  (leave this line)
        "https://api-rinkeby.etherscan.io", // etherscan endpoint_url
        "0x8d9Eda359157594F352dc29c0bDB741bb8F6b65e", // contract_addr
        "" // etherscan your api token (not necessary)
    )
    .await?
    .as_bytes(),
     "store", // method_name
     0_u32, // args
    "ebefaa0570e26ce96cf0876ff68648027de39b30119b16953aa93e73d35064c1" // private_key
    ).await?; 
    println!("Tx: {}", change_response);
    Ok(())
}

```

TO RUN the example, create an "examples" folder in the "eth_client" folder, then create a "contract_estimate_transfer_execution.rs" file and put the code below in it, changing the parameter values to your own, then write in the CLI "cargo run --example contract_estimate_transfer_execution"

## Example of calling estimation transfer execution method

```rust

use eth_client::methods::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let estimated_gas_in_wei = estimate_gas(
        "https://rinkeby.infura.io/v3/168bdff2f03e417eb8e69cd90fc54615",
        "0x8d9Eda359157594F352dc29c0bDB741bb8F6b65e",
        get_contract_abi(
            "https://api-rinkeby.etherscan.io",
            "0x8d9Eda359157594F352dc29c0bDB741bb8F6b65e",
            "",
        )
        .await?
        .as_bytes(),
        "store",
        0_u32,
    )
    .await?;
    println!("estimated_gas_in_wei: {}", estimated_gas_in_wei);
    let gas_price_in_wei =
        gas_price("https://rinkeby.infura.io/v3/168bdff2f03e417eb8e69cd90fc54615").await?;
    println!("gas_price_in_wei: {}", gas_price_in_wei);
    let ether_in_usd = eth_price().await?;
    println!(
        "estimate_transfer_execution (usd): {}",
        estimate_transfer_execution(estimated_gas_in_wei, gas_price_in_wei, ether_in_usd)
    );
    Ok(())
}

```
