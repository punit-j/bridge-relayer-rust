use web3::{contract::Options, types::BlockNumber};

const EIP_1559_TRANSACTION_TYPE: u64 = 2;

pub fn new_eth_rpc_client(timeout: Option<std::time::Duration>) -> web3::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();
    builder = builder.user_agent(reqwest::header::HeaderValue::from_static("web3.rs"));

    if let Some(timeout) = timeout {
        builder = builder.timeout(timeout).connect_timeout(timeout);
    }

    Ok(builder.build().map_err(|err| {
        web3::Error::Transport(web3::error::TransportError::Message(format!(
            "failed to build client: {}",
            err
        )))
    })?)
}

pub fn construct_contract_interface(
    eth_endpoint: reqwest::Url,
    contract_addr: web3::types::Address,
    contract_abi: &[u8],
    rpc_timeout_secs: u64,
) -> web3::contract::Result<web3::contract::Contract<web3::transports::Http>> {
    let transport = web3::transports::Http::with_client(
        new_eth_rpc_client(Some(std::time::Duration::from_secs(rpc_timeout_secs)))?,
        eth_endpoint,
    );
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

struct FeeData {
    #[allow(dead_code)]
    base_fee_per_gas: web3::types::U256,
    max_priority_fee_per_gas: web3::types::U256,
    max_fee_per_gas: web3::types::U256,
}

async fn get_fee_data(
    server_address: reqwest::Url,
    max_priority_fee_per_gas: Option<web3::types::U256>,
    rpc_timeout_secs: u64,
) -> web3::contract::Result<FeeData> {
    let transport = web3::transports::Http::with_client(
        new_eth_rpc_client(Some(std::time::Duration::from_secs(rpc_timeout_secs)))?,
        server_address,
    );
    let client = web3::Web3::new(transport);

    let last_block = client
        .eth()
        .block(web3::types::BlockId::Number(
            web3::types::BlockNumber::Latest,
        ))
        .await?
        .ok_or("Failed to get last block".to_string())?;

    let base_fee_per_gas = last_block
        .base_fee_per_gas
        .ok_or("Failed to get `base_fee_per_gas`".to_string())?;
    let max_priority_fee_per_gas: web3::types::U256 =
        max_priority_fee_per_gas.unwrap_or(1500000000.into());
    let max_fee_per_gas = base_fee_per_gas
        .checked_mul(2.into())
        .ok_or("Failed to calculate `max_fee_per_gas`".to_string())?
        .checked_add(max_priority_fee_per_gas)
        .ok_or("Failed to calculate `max_fee_per_gas`".to_string())?;

    Ok(FeeData {
        base_fee_per_gas,
        max_priority_fee_per_gas,
        max_fee_per_gas,
    })
}

pub async fn get_transaction_count(
    server_address: &str,
    account_address: web3::types::Address,
) -> web3::error::Result<web3::types::U256> {
    let transport = web3::transports::Http::new(server_address)?;
    let client = web3::Web3::new(transport);
    client
        .eth()
        .transaction_count(account_address, Some(BlockNumber::Pending))
        .await
}

pub async fn change(
    server_addr: reqwest::Url,
    contract_addr: web3::types::Address,
    contract_abi: &[u8],
    method_name: &str,
    args: impl web3::contract::tokens::Tokenize,
    key: impl web3::signing::Key,
    use_eip_1559: bool,
    transaction_count: Option<web3::types::U256>,
    gas: Option<web3::types::U256>,
    max_priority_fee_per_gas: Option<web3::types::U256>,
    rpc_timeout_secs: u64,
) -> web3::contract::Result<web3::types::H256> {
    let mut options = web3::contract::Options::default();
    options.nonce = transaction_count;

    if use_eip_1559 {
        let fee_data = get_fee_data(
            server_addr.clone(),
            max_priority_fee_per_gas,
            rpc_timeout_secs,
        )
        .await?;
        options.max_fee_per_gas = Some(fee_data.max_fee_per_gas);
        options.max_priority_fee_per_gas = Some(fee_data.max_priority_fee_per_gas);
        options.transaction_type = Some(EIP_1559_TRANSACTION_TYPE.into());
        options.gas = gas;
    }

    let abi =
        construct_contract_interface(server_addr, contract_addr, contract_abi, rpc_timeout_secs)?;
    Ok(abi.signed_call(method_name, args, options, key).await?)
}

pub async fn gas_price_wei(server_addr: &str) -> web3::contract::Result<web3::types::U256> {
    let transport = web3::transports::Http::new(server_addr)?;
    let client = web3::Web3::new(transport);
    Ok(client.eth().gas_price().await?)
}

pub async fn estimate_gas(
    eth_endpoint: reqwest::Url,
    signer_eth_addr: web3::types::Address,
    contract_addr: web3::types::Address,
    contract_abi: &[u8],
    method_name: &str,
    args: impl web3::contract::tokens::Tokenize,
    rpc_timeout_secs: u64,
) -> web3::contract::Result<web3::types::U256> {
    let abi =
        construct_contract_interface(eth_endpoint, contract_addr, contract_abi, rpc_timeout_secs)?;
    abi.estimate_gas(method_name, args, signer_eth_addr, Options::default())
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
    client.ping().await?;
    let token_price = client
        .price(&[&coin_id], &["usd"], true, true, true, true)
        .await;

    match token_price?.get(&coin_id) {
        Some(entry) => Ok(entry.usd),
        None => Ok(None),
    }
}

pub async fn get_last_block_number(server_addr: &str) -> web3::contract::Result<u64> {
    let transport = web3::transports::Http::new(server_addr)?;
    let client = web3::Web3::new(transport);
    Ok(client.eth().block_number().await?.as_u64())
}

#[cfg(test)]
pub mod tests {
    use crate::methods::{
        change, construct_contract_interface, estimate_gas, estimate_transfer_execution_usd,
        eth_price_usd, gas_price_wei, get_fee_data, get_transaction_count, token_price_usd,
    };
    use crate::test_utils;
    use crate::test_utils::{
        get_eth_contract_abi, get_eth_erc20_fast_bridge_contract_abi,
        get_eth_erc20_fast_bridge_proxy_contract_address, get_eth_rpc_url, get_eth_token,
        get_recipient, get_relay_eth_key,
    };
    use std::str::FromStr;
    use url::Url;
    use web3::types::{Address, U256};

    #[tokio::test]
    async fn smoke_estimate_gas_test() {
        let contract_abi = test_utils::get_eth_erc20_fast_bridge_contract_abi().await;
        let eth1_endpoint = test_utils::get_eth_rpc_url();

        let bridge_proxy_addres = test_utils::get_eth_erc20_fast_bridge_proxy_contract_address();
        let signer_addres = bridge_proxy_addres.clone();
        let method_name = "isTokenInWhitelist";

        let token = test_utils::get_eth_token();
        let method_args = token;

        let estimated_gas = estimate_gas(
            eth1_endpoint,
            signer_addres,
            bridge_proxy_addres,
            contract_abi.as_bytes(),
            method_name,
            method_args,
            30,
        )
        .await
        .unwrap();

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

        let estimated_price =
            estimate_transfer_execution_usd(estimated_gas, gas_price, ether_price);

        println!("Estimated transfer execution = {}$", estimated_price);
        assert!(estimated_price > 1.8);
        assert!(estimated_price < 1.9);
    }

    #[tokio::test]
    async fn smoke_token_price_test() {
        let token_name = "aurora-near";
        let token_price = token_price_usd(token_name.to_string())
            .await
            .unwrap()
            .unwrap();
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
        let eth1_endpoint = get_eth_rpc_url();

        let bridge_proxy_addres = get_eth_erc20_fast_bridge_proxy_contract_address();
        let contract_abi = get_eth_erc20_fast_bridge_contract_abi().await;

        let method_name = "transferTokens";

        let token = get_eth_token();
        let recipient = get_recipient();
        let nonce = web3::types::U256::from(200);
        let amount = web3::types::U256::from(1);

        let method_args = (token, recipient, nonce, amount);

        let priv_key = get_relay_eth_key();

        let res = change(
            eth1_endpoint,
            bridge_proxy_addres,
            contract_abi.as_bytes(),
            &method_name,
            method_args,
            &priv_key,
            true,
            None,
            None,
            None,
            30,
        )
        .await
        .unwrap();

        println!("transaction hash: {:?}", res);
    }

    #[tokio::test]
    async fn mint_token() {
        let eth1_endpoint = get_eth_rpc_url();
        let token = get_eth_token();

        let contract_abi = get_eth_contract_abi(token).await;

        let method_name = "mint";
        let amount = web3::types::U256::from(100);

        let priv_key = get_relay_eth_key();

        let res = change(
            eth1_endpoint,
            token,
            contract_abi.as_bytes(),
            &method_name,
            amount,
            &priv_key,
            true,
            None,
            None,
            None,
            30,
        )
        .await
        .unwrap();

        println!("transaction hash: {:?}", res);
    }

    #[tokio::test]
    async fn test_construct_contract_interface_not_eth_endpoint_url() {
        let contract_abi = test_utils::get_eth_erc20_fast_bridge_contract_abi().await;
        let bridge_proxy_addres = test_utils::get_eth_erc20_fast_bridge_proxy_contract_address();

        construct_contract_interface(
            Url::from_str("https://www.google.com/").unwrap(),
            bridge_proxy_addres,
            contract_abi.as_bytes(),
            30,
        )
        .unwrap();
    }

    #[tokio::test]
    #[should_panic = "Abi(SerdeJson(Error(\"invalid type: map, expected valid abi spec file\", line: 1, column: 1)))"]
    async fn test_construct_contract_interface_incorrect_json() {
        let eth1_endpoint = test_utils::get_eth_rpc_url();
        let bridge_proxy_addres = test_utils::get_eth_erc20_fast_bridge_proxy_contract_address();

        construct_contract_interface(eth1_endpoint, bridge_proxy_addres, "{".as_bytes(), 30)
            .unwrap();
    }

    #[tokio::test]
    #[should_panic = "Abi(SerdeJson(Error(\"invalid type: map, expected valid abi spec file\", line: 1, column: 2)))"]
    async fn test_construct_contract_interface_not_abi_json() {
        let eth1_endpoint = test_utils::get_eth_rpc_url();
        let bridge_proxy_addres = test_utils::get_eth_erc20_fast_bridge_proxy_contract_address();

        construct_contract_interface(eth1_endpoint, bridge_proxy_addres, "{}".as_bytes(), 30)
            .unwrap();
    }

    #[tokio::test]
    async fn test_construct_contract_interface_non_existing_address() {
        let contract_abi = test_utils::get_eth_erc20_fast_bridge_contract_abi().await;
        let eth1_endpoint = test_utils::get_eth_rpc_url();
        let bridge_proxy_addres = web3::types::Address::from_slice(
            hex::decode("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
                .unwrap()
                .as_slice(),
        );

        construct_contract_interface(
            eth1_endpoint,
            bridge_proxy_addres,
            contract_abi.as_bytes(),
            30,
        )
        .unwrap();
    }

    #[tokio::test]
    async fn test_construct_contract_interface_unreachable_url() {
        let contract_abi = test_utils::get_eth_erc20_fast_bridge_contract_abi().await;

        let bridge_proxy_addres = test_utils::get_eth_erc20_fast_bridge_proxy_contract_address();

        construct_contract_interface(
            Url::from_str("http://httpstat.us/404").unwrap(),
            bridge_proxy_addres,
            contract_abi.as_bytes(),
            30,
        )
        .unwrap();
    }

    #[tokio::test]
    #[should_panic = "Api(Transport(Code(404)))"]
    async fn test_get_fee_data_bad_server() {
        get_fee_data(Url::from_str("http://httpstat.us/404").unwrap(), None, 30)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic = "InvalidOutputType(\"Failed to calculate `max_fee_per_gas`\")"]
    async fn test_get_fee_data_big_max_priority() {
        let eth1_endpoint = test_utils::get_eth_rpc_url();

        get_fee_data(eth1_endpoint, Some(U256::MAX), 30)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic = "Transport(Code(404))"]
    async fn test_transaction_count_bad_server() {
        let eth_addres = test_utils::get_eth_token();

        get_transaction_count("http://httpstat.us/404", eth_addres)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_transaction_count_bad_account() {
        let eth1_endpoint = test_utils::get_eth_rpc_url().to_string();

        assert_eq!(
            U256::zero(),
            get_transaction_count(&eth1_endpoint, Address::zero())
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    #[should_panic = "Api(Transport(Code(404)))"]
    async fn test_change_bad_server_not_use_eip_1559() {
        let token = get_eth_token();

        let contract_abi = get_eth_contract_abi(token).await;

        let method_name = "mint";
        let amount = web3::types::U256::from(100);

        let priv_key = get_relay_eth_key();

        change(
            Url::from_str("http://httpstat.us/404").unwrap(),
            token,
            contract_abi.as_bytes(),
            &method_name,
            amount,
            &priv_key,
            false,
            None,
            None,
            None,
            30,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    #[should_panic = "Api(Transport(Code(404)))"]
    async fn test_change_bad_server_use_eip_1559() {
        let token = get_eth_token();

        let contract_abi = get_eth_contract_abi(token).await;

        let method_name = "mint";
        let amount = web3::types::U256::from(100);

        let priv_key = get_relay_eth_key();

        change(
            Url::from_str("http://httpstat.us/404").unwrap(),
            token,
            contract_abi.as_bytes(),
            &method_name,
            amount,
            &priv_key,
            true,
            None,
            None,
            None,
            30,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_change_invalid_address() {
        let eth1_endpoint = get_eth_rpc_url();

        let token = get_eth_token();
        let contract_abi = get_eth_contract_abi(token).await;
        let token = web3::types::Address::from_slice(
            hex::decode("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
                .unwrap()
                .as_slice(),
        );

        let method_name = "mint";
        let amount = web3::types::U256::from(100);

        let priv_key = get_relay_eth_key();

        change(
            eth1_endpoint,
            token,
            contract_abi.as_bytes(),
            &method_name,
            amount,
            &priv_key,
            true,
            None,
            None,
            None,
            30,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    #[should_panic = "Api(Decoder(\"InvalidName(\\\"min\\\")\"))"]
    async fn test_change_wrong_method_name() {
        let eth1_endpoint = get_eth_rpc_url();

        let token = get_eth_token();
        let contract_abi = get_eth_contract_abi(token).await;

        let method_name = "min";
        let amount = web3::types::U256::from(100);

        let priv_key = get_relay_eth_key();

        change(
            eth1_endpoint,
            token,
            contract_abi.as_bytes(),
            &method_name,
            amount,
            &priv_key,
            true,
            None,
            None,
            None,
            30,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    #[should_panic = "Api(Decoder(\"InvalidData\"))"]
    async fn test_change_wrong_method_args() {
        let eth1_endpoint = get_eth_rpc_url();

        let token = get_eth_token();
        let contract_abi = get_eth_contract_abi(token).await;

        let method_name = "mint";
        let amount = 100;

        let priv_key = get_relay_eth_key();

        change(
            eth1_endpoint,
            token,
            contract_abi.as_bytes(),
            &method_name,
            amount,
            &priv_key,
            true,
            None,
            None,
            None,
            30,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    #[should_panic = "Api(Rpc(Error { code: ServerError(-32000), message: \"insufficient funds for gas * price + value\", data: None }))"]
    async fn test_change_wrong_private_key() {
        let eth1_endpoint = get_eth_rpc_url();

        let token = get_eth_token();
        let contract_abi = get_eth_contract_abi(token).await;

        let method_name = "mint";
        let amount = web3::types::U256::from(100);

        let priv_key = secp256k1::SecretKey::from_str(
            "0000090000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();

        change(
            eth1_endpoint,
            token,
            contract_abi.as_bytes(),
            &method_name,
            amount,
            &priv_key,
            true,
            None,
            None,
            None,
            30,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    #[should_panic = "Api(Rpc(Error { code: ServerError(-32000), message: \"replacement transaction underpriced\", data: None }))"]
    async fn test_change_wrong_nonce() {
        let eth1_endpoint = get_eth_rpc_url();

        let token = get_eth_token();
        let contract_abi = get_eth_contract_abi(token).await;

        let method_name = "mint";
        let amount = web3::types::U256::from(100);

        let priv_key = get_relay_eth_key();

        change(
            eth1_endpoint.clone(),
            token,
            contract_abi.as_bytes(),
            &method_name,
            amount,
            &priv_key,
            true,
            None,
            None,
            None,
            30,
        )
        .await
        .unwrap();

        let amount = web3::types::U256::from(200);

        change(
            eth1_endpoint,
            token,
            contract_abi.as_bytes(),
            &method_name,
            amount,
            &priv_key,
            true,
            None,
            None,
            None,
            30,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    #[should_panic = "Api(Transport(Code(404)))"]
    async fn test_get_price_wei_incorrect_server_address() {
        gas_price_wei("http://httpstat.us/404").await.unwrap();
    }

    #[tokio::test]
    #[should_panic = "Api(Transport(Code(404)))"]
    async fn test_gas_estimation_bad_server() {
        let token = get_eth_token();
        let contract_abi = get_eth_contract_abi(token).await;

        let method_name = "mint";
        let amount = web3::types::U256::from(100);

        let priv_key = get_eth_erc20_fast_bridge_proxy_contract_address();

        estimate_gas(
            Url::from_str("http://httpstat.us/404").unwrap(),
            priv_key,
            token,
            contract_abi.as_bytes(),
            method_name,
            amount,
            30,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    #[should_panic = "Abi(InvalidName(\"min\"))"]
    async fn test_gas_estimation_wrong_method_name() {
        let eth1_endpoint = get_eth_rpc_url();

        let token = get_eth_token();
        let contract_abi = get_eth_contract_abi(token).await;

        let method_name = "min";
        let amount = web3::types::U256::from(100);

        let priv_key = get_eth_erc20_fast_bridge_proxy_contract_address();

        estimate_gas(
            eth1_endpoint,
            priv_key,
            token,
            contract_abi.as_bytes(),
            method_name,
            amount,
            30,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    #[should_panic = "Abi(InvalidData)"]
    async fn test_gas_estimation_wrong_args() {
        let eth1_endpoint = get_eth_rpc_url();

        let token = get_eth_token();
        let contract_abi = get_eth_contract_abi(token).await;

        let method_name = "mint";
        let amount = 100;

        let priv_key = get_eth_erc20_fast_bridge_proxy_contract_address();

        estimate_gas(
            eth1_endpoint,
            priv_key,
            token,
            contract_abi.as_bytes(),
            method_name,
            amount,
            30,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    #[should_panic = "Api(Rpc(Error { code: ServerError(3), message: \"execution reverted: ERC20: mint to the zero address\""]
    async fn test_gas_estimation_wrong_eth_address() {
        let eth1_endpoint = get_eth_rpc_url();

        let token = get_eth_token();
        let contract_abi = get_eth_contract_abi(token).await;

        let method_name = "mint";
        let amount = web3::types::U256::from(100);

        let priv_key = Address::zero();

        estimate_gas(
            eth1_endpoint,
            priv_key,
            token,
            contract_abi.as_bytes(),
            method_name,
            amount,
            30,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_token_price_incorrect_token_name() {
        let token_name = "unexistingtoken";
        let token_price = token_price_usd(token_name.to_string()).await.unwrap();
        assert_eq!(token_price, None);
    }

    #[test]
    #[should_panic]
    fn test_estimate_transfer_execution_usd_max() {
        estimate_transfer_execution_usd(U256::MAX, U256::MAX, f64::MAX);
    }

    #[test]
    fn test_estimate_transfer_execution_usd_max_possible() {
        estimate_transfer_execution_usd(
            U256::from(100_000_000),
            U256::from(40_000_000_u128 * 1_000_000_000_u128),
            10_000.,
        );
    }
}
