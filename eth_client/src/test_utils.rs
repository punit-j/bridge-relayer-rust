use crate::methods::get_contract_abi;
use std::env;
use std::str::FromStr;
use web3::types::Address;
use url::Url;

const ETH_CONTRACT_PROXY_ADDRESS: &str = "8AC4c4A1015A9A12A9DBA16234A3f7909b9396Eb";
const ETH_CONTRACT_IMPLEMENTATION_ADDRESS: &str = "B6b5739c390648A0121502ab3c3F4112f3FeAc1a";
pub const ETH_TOKEN_ADDRESS: &str = "b2d75C5a142A68BDA438e6a318C7FBB2242f9693";

pub async fn get_eth_contract_abi(contract_addr: Address) -> String {
    let etherscan_endpoint_url = "https://api-goerli.etherscan.io";
    let etherscan_api_key = env::var("ETHERSCAN_API_KEY").unwrap();
    get_contract_abi(
        etherscan_endpoint_url,
        contract_addr,
        &etherscan_api_key,
    )
        .await
        .unwrap()
}

pub async fn get_eth_erc20_fast_bridge_contract_abi() -> String {
    let eth_bridge_impl_address = web3::types::Address::from_slice(
        hex::decode(ETH_CONTRACT_IMPLEMENTATION_ADDRESS)
            .unwrap()
            .as_slice(),
    );

    get_eth_contract_abi(eth_bridge_impl_address).await
}

pub fn get_eth_erc20_fast_bridge_proxy_contract_address() -> web3::types::Address {
    web3::types::Address::from_slice(
        hex::decode(ETH_CONTRACT_PROXY_ADDRESS)
            .unwrap()
            .as_slice(),
    )
}

pub fn get_relay_eth_key() -> secp256k1::SecretKey {
    secp256k1::SecretKey::from_str(&(env::var("SPECTRE_BRIDGE_ETH_PRIVATE_KEY").unwrap())[..64])
        .unwrap()
}

pub fn get_eth_rpc_url() -> Url {
    let api_key_string = env::var("ETH_GOERLI_INFURA_API_KEY").unwrap();
    url::Url::parse(&format!("https://goerli.infura.io/v3/{}", &api_key_string)).unwrap()
}

pub fn get_eth_token() -> web3::types::Address {
    web3::types::Address::from_str(ETH_TOKEN_ADDRESS).unwrap()
}

pub fn get_recipient() -> web3::types::Address {
    web3::types::Address::from_str("2a23E0Fa3Afe77AFf5dc6c6a007E3A10c1450633").unwrap()
}