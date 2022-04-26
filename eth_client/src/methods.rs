use std::str::FromStr;

pub fn construct_contract_interface(
    server_addr: &str,
    contract_addr: &str,
    contract_abi: &[u8],
) -> web3::contract::Result<web3::contract::Contract<web3::transports::Http>> {
    let transport = web3::transports::Http::new(server_addr)?;
    let client = web3::Web3::new(transport);
    Ok(web3::contract::Contract::from_json(
        client.eth(),
        contract_addr.parse().unwrap(),
        contract_abi,
    )?)
}

// Alternative to this feature: include_bytes!("./ABSOLUTEPATH/FILENAME.abi")
pub async fn get_contract_abi(
    endpoint_url: &str,
    contract_addr: &str,
    api_key_token: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let response = reqwest::get(format!(
        "{}/api?module=contract&action=getabi&address={}&apikey={}",
        endpoint_url, contract_addr, api_key_token
    ))
    .await?
    .text()
    .await?;
    let mut response: serde_json::Value = serde_json::from_str(&response).expect("Unable to parse");
    let response = response["result"].take().to_string().replace("\\", "");
    Ok(response[1..response.len() - 1].to_string())
}

pub async fn change(
    server_addr: &str,
    contract_addr: &str,
    contract_abi: &[u8],
    method_name: &str,
    args: impl web3::contract::tokens::Tokenize,
    private_key: &str,
) -> web3::contract::Result<web3::types::H256> {
    let abi = construct_contract_interface(server_addr, contract_addr, contract_abi)?;
    Ok(abi
        .signed_call(
            method_name,
            args,
            web3::contract::Options::default(),
            &secp256k1::SecretKey::from_str(private_key).unwrap(),
        )
        .await?)
}

pub async fn gas_price(server_addr: &str) -> web3::contract::Result<web3::types::U256> {
    let transport = web3::transports::Http::new(server_addr)?;
    let client = web3::Web3::new(transport);
    Ok(client.eth().gas_price().await?)
}

pub async fn estimate_gas(
    server_addr: &str,
    contract_addr: &str,
    contract_abi: &[u8],
    method_name: &str,
    args: impl web3::contract::tokens::Tokenize,
) -> web3::contract::Result<web3::types::U256> {
    let abi = construct_contract_interface(server_addr, contract_addr, contract_abi)?;
    Ok(abi
        .estimate_gas(
            method_name,
            args,
            contract_addr.parse().unwrap(),
            web3::contract::Options::default(),
        )
        .await?)
}

pub async fn eth_price() -> Result<f64, reqwest::Error> {
    let client = coingecko::CoinGeckoClient::default();
    Ok(client
        .price(&["ethereum"], &["usd"], true, true, true, true)
        .await?
        .get("ethereum")
        .unwrap()
        .usd
        .unwrap())
}

pub fn estimate_transfer_execution(
    estimated_gas: web3::types::U256,
    gas_price: web3::types::U256,
    ether_price: f64,
) -> f64 {
    let ether_in_wei: web3::types::U256 = web3::types::U256::from(1_000_000_000_000_000_000u64);
    let precision = u32::pow(10, 4) as f64;
    let ether_price = web3::types::U256::from((ether_price * precision) as u64);
    estimated_gas
        .checked_mul(gas_price)
        .unwrap()
        .checked_mul(ether_price)
        .unwrap()
        .checked_div(ether_in_wei)
        .unwrap()
        .as_u64() as f64
        / precision
}