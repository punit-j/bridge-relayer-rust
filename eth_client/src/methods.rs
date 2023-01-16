#![allow(unused_imports)]
use std::str::FromStr;

pub fn construct_contract_interface(
    eth_endpoint: &str,
    contract_addr: web3::types::Address,
    contract_abi: &[u8],
) -> web3::contract::Result<web3::contract::Contract<web3::transports::Http>> {
    let transport = web3::transports::Http::new(eth_endpoint)?;
    let client = web3::Web3::new(transport);
    Ok(web3::contract::Contract::from_json(
        client.eth(),
        contract_addr,
        contract_abi,
    )?)
}

// Alternative to this feature: include_bytes!("./<PATH>/<FILENAME.abi>")
pub async fn get_contract_abi(
    endpoint_url: &str,
    contract_addr: web3::types::Address,
    api_key_token: &str,
) -> Result<String, String> {
    let response = reqwest::get(format!(
        "{}/api?module=contract&action=getabi&address={:?}&apikey={}&format=raw",
        endpoint_url, contract_addr, api_key_token
    ))
    .await
    .map_err(|e| e.to_string())?
    .text()
    .await
    .map_err(|e| e.to_string())?;

    #[derive(Clone, Debug, PartialEq, serde::Deserialize)]
    struct ErrResponse {
        message: String,
        result: String,
    }

    // try to get error
    if let Ok(res) = serde_json::from_str::<ErrResponse>(response.as_str()) {
        if res.message == "NOTOK" {
            return Err(res.result);
        }
    }

    Ok(response)
}

pub async fn change(
    server_addr: &str,
    contract_addr: web3::types::Address,
    contract_abi: &[u8],
    method_name: &str,
    args: impl web3::contract::tokens::Tokenize,
    key: impl web3::signing::Key,
) -> web3::contract::Result<web3::types::H256> {
    let abi = construct_contract_interface(server_addr, contract_addr, contract_abi)?;
    Ok(abi
        .signed_call(method_name, args, web3::contract::Options::default(), key)
        .await?)
}

pub async fn change_with_confirmations(
    server_addr: &str,
    contract_addr: web3::types::Address,
    contract_abi: &[u8],
    method_name: &str,
    args: impl web3::contract::tokens::Tokenize,
    confirmations: usize,
    key: impl web3::signing::Key,
) -> web3::contract::Result<web3::types::TransactionReceipt> {
    let abi = construct_contract_interface(server_addr, contract_addr, contract_abi)?;
    Ok(abi
        .signed_call_with_confirmations(method_name, args, web3::contract::Options::default(), confirmations, key)
        .await?)
}

pub async fn gas_price_wei(server_addr: &str) -> web3::contract::Result<web3::types::U256> {
    let transport = web3::transports::Http::new(server_addr)?;
    let client = web3::Web3::new(transport);
    Ok(client.eth().gas_price().await?)
}

pub async fn estimate_gas(
    eth_endpoint: &str,
    signer_eth_addr: web3::types::Address,
    contract_eth_addr: web3::types::Address,
    contract_abi: &[u8],
    method_name: &str,
    args: impl web3::contract::tokens::Tokenize,
) -> web3::contract::Result<web3::types::U256> {
    let abi = construct_contract_interface(eth_endpoint, contract_eth_addr, contract_abi)?;
    abi.estimate_gas(
        method_name,
        args,
        signer_eth_addr,
        web3::contract::Options::default(),
    )
    .await
}

pub fn estimate_transfer_execution_usd(
    estimated_gas: web3::types::U256,
    gas_price_wei: web3::types::U256,
    ether_price_usd: f64,
) -> f64 {
    let precision = f64::powf(10.0, 4.0);
    let ether_in_wei = web3::types::U256::from(1_000_000_000_000_000_000u64);
    let ether_price = web3::types::U256::from((ether_price_usd * precision) as u128);
    estimated_gas
        .checked_mul(gas_price_wei)
        .unwrap()
        .checked_mul(ether_price)
        .unwrap()
        .checked_div(ether_in_wei)
        .unwrap()
        .as_u64() as f64
        / precision
}

pub async fn eth_price_usd() -> Result<Option<f64>, reqwest::Error> {
    token_price_usd("ethereum".to_string()).await
}

pub async fn token_price_usd(coin_id: String) -> Result<Option<f64>, reqwest::Error> {
    let client = coingecko::CoinGeckoClient::default();
    match client.ping().await {
        Ok(_) => {
            let token_price = client
                .price(&[&coin_id], &["usd"], true, true, true, true)
                .await;
            match token_price {
                Ok(hashmap) => match hashmap.get(&coin_id) {
                    Some(entry) => Ok(entry.usd),
                    None => Ok(None),
                },
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
pub mod tests {
    use crate::methods::{estimate_transfer_execution_usd, eth_price_usd, gas_price_wei, token_price_usd, get_contract_abi, estimate_gas, change};
    use std::str::FromStr;
    use std::env;
    use crate::test_utils;
    use crate::test_utils::{get_eth_contract_abi, get_eth_erc20_fast_bridge_contract_abi, get_eth_erc20_fast_bridge_proxy_contract_address, get_eth_rpc_url, get_eth_token, get_recipient, get_relay_eth_key};

    #[tokio::test]
    async fn smoke_estimate_gas_test() {
        let contract_abi = test_utils::get_eth_erc20_fast_bridge_contract_abi().await;
        let eth1_endpoint = test_utils::get_eth_rpc_url().to_string();

        let bridge_proxy_addres = test_utils::get_eth_erc20_fast_bridge_proxy_contract_address();
        let signer_addres = bridge_proxy_addres.clone();
        let method_name = "isTokenInWhitelist";

        let token = test_utils::get_eth_token();
        let method_args = token;

        let estimated_gas = estimate_gas(
            &eth1_endpoint,
            signer_addres,
            bridge_proxy_addres,
            contract_abi.as_bytes(),
            method_name,
            method_args
        ).await.unwrap();

        println!("Estimated gas = {}", estimated_gas);
    }

    #[tokio::test]
    async fn smoke_gas_price_test() {
        let eth1_endpoint = get_eth_rpc_url().to_string();
        const GWEI_IN_WEI: u64 = 1_000_000_000;
        const MAX_PRICE_IN_GWEI: u64 = 1_000_000_000;
        const MIN_PRICE_IN_WEI: u64 = 0;

        if let Ok(gas_price) = gas_price_wei(&eth1_endpoint).await {
            println!("Current gas price = {}", gas_price);

            assert!(gas_price >= web3::types::U256::from(MIN_PRICE_IN_WEI));
            assert!(gas_price <= web3::types::U256::from(GWEI_IN_WEI * MAX_PRICE_IN_GWEI));
        } else {
            panic!("Error on fetching gas price!");
        }
    }

    #[tokio::test]
    // data from some random transaction https://etherscan.io/tx/0xd4e7e8c91f43f13202f647efb726e867f2ae44a8d633fe5ad2549de03f1496c6
    async fn smoke_estimate_transfer_execution_test() {
        let estimated_gas: web3::types::U256 = web3::types::U256::from(116_855);
        let gas_price: web3::types::U256 = web3::types::U256::from(13_088_907_561 as i64);
        let ether_price: f64 = 1208.69;

        let estimated_price = estimate_transfer_execution_usd(estimated_gas, gas_price, ether_price);

        println!("Estimated transfer execution = {}$", estimated_price);
        assert!(estimated_price > 1.8);
        assert!(estimated_price < 1.9);
    }

    #[tokio::test]
    async fn smoke_token_price_test() {
        let token_name = "aurora-near";
        let token_price = token_price_usd(token_name.to_string()).await.unwrap().unwrap();
        println!("{} token price usd = {}", token_name, token_price);
        assert!(token_price > 0.);
        assert!(token_price < 1_000_000.);
    }

    #[tokio::test]
    async fn smoke_eth_price_test() {
        if let Ok(Some(eth_price)) = eth_price_usd().await {
            println!("eth price usd = {}", eth_price);
            assert!(eth_price > 0.);
            assert!(eth_price < 1_000_000.);
        } else {
            panic!("Error during fetching ETH price!");
        }
    }

    #[tokio::test]
    async fn smoke_change_test() {
        let eth1_endpoint = get_eth_rpc_url().to_string();

        let bridge_proxy_addres = get_eth_erc20_fast_bridge_proxy_contract_address();
        let contract_abi = get_eth_erc20_fast_bridge_contract_abi().await;

        let method_name = "transferTokens";

        let token = get_eth_token();
        let recipient = get_recipient();
        let nonce = web3::types::U256::from(200);
        let amount = web3::types::U256::from(1);

        let method_args = (token, recipient, nonce, amount);

        let priv_key = get_relay_eth_key();

        let res = change(&eth1_endpoint, bridge_proxy_addres, contract_abi.as_bytes(), &method_name, method_args, &priv_key).await.unwrap();

        println!("transaction hash: {:?}", res);
    }

    #[tokio::test]
    async fn mint_token() {
        let eth1_endpoint = get_eth_rpc_url().to_string();
        let token = get_eth_token();

        let contract_abi = get_eth_contract_abi(token).await;

        let method_name = "mint";
        let amount = web3::types::U256::from(100);

        let priv_key = get_relay_eth_key();

        let res = change(&eth1_endpoint, token, contract_abi.as_bytes(), &method_name, amount, &priv_key).await.unwrap();

        println!("transaction hash: {:?}", res);
    }
}
