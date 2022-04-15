use secp256k1::SecretKey;
use std::str::FromStr;

pub fn construct_contract_interface(
    server_addr: &str,
    contract_addr: &str,
    contract_abi: &[u8],
) -> web3::contract::Result<web3::contract::Contract<web3::transports::Http>> {
    let transport = web3::transports::Http::new(server_addr)?;
    let web3 = web3::Web3::new(transport);
    Ok(web3::contract::Contract::from_json(
        web3.eth(),
        contract_addr.parse().unwrap(),
        contract_abi,
    )?)
}

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
    Ok(
        construct_contract_interface(server_addr, contract_addr, contract_abi)?
            .signed_call(
                method_name,
                args,
                web3::contract::Options::default(),
                &SecretKey::from_str(private_key).unwrap(),
            )
            .await?,
    )
}
