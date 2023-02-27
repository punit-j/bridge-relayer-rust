use crate::config::Settings;
use std::{env::var, str::FromStr};

use http_client::HttpClient;
use http_types::{Method, Request};
use near_crypto::InMemorySigner;

#[allow(unused)]
const NEAR_VAULT_PREFIX: &str = "nearsignerSigningKey";
const ETH_VAULT_PREFIX: &str = "ethsignerSigningKey";

async fn read_secret_key(
    settings: &Settings,
    vault_token: &String,
    prefix: &str,
    key: &str,
) -> String {
    let client = http_client::h1::H1Client::new();
    let mut addr = settings.vault_addr.to_string();
    addr.push_str(prefix);

    let mut request = Request::new(Method::Get, addr.as_str());
    request.insert_header("X-Vault-Token", vault_token);

    let mut response = client.send(request).await.unwrap();
    let res: serde_json::Value =
        serde_json::from_str(&response.body_string().await.unwrap()).unwrap();

    match res.get("data") {
        None => {
            let err: String = res
                .get("errors")
                .expect("Cannot parse errors field in Vault response")
                .to_string();
            println!("Error while try to read from Vault: {}", err);
            String::new()
        }
        Some(data) => data
            .get("data")
            .expect("Cannot read private key from Vault response")
            .get(key)
            .expect("Cannot read private key from Vault response")
            .to_string()
            .replace(&['\"'], ""),
    }
}

#[allow(unused)]
pub struct NearKey {}

#[allow(unused)]
impl NearKey {
    pub async fn vault_account_id(settings: &Settings, vault_token: &String) -> String {
        read_secret_key(settings, vault_token, NEAR_VAULT_PREFIX, "account_id").await
    }

    pub async fn vault_private_key(settings: &Settings, vault_token: &String) -> String {
        read_secret_key(settings, vault_token, NEAR_VAULT_PREFIX, "key").await
    }

    pub async fn get_signer(settings: &Settings, vault_token: Option<String>) -> InMemorySigner {
        let vault_token = vault_token.unwrap_or(var("VAULT_TOKEN").unwrap());

        let private_key = NearKey::vault_private_key(settings, &vault_token).await;
        let account_id = NearKey::vault_account_id(settings, &vault_token).await;

        let private_key = near_crypto::SecretKey::from_str(private_key.as_str())
            .expect("Error in parse private key");
        let account_id = near_primitives::types::AccountId::from_str(account_id.as_str())
            .expect("Error in parse Near account id");

        near_crypto::InMemorySigner::from_secret_key(account_id, private_key)
    }
}

pub struct EthKey {}

impl EthKey {
    pub async fn vault_private_key(settings: &Settings, vault_token: Option<String>) -> String {
        let vault_token = vault_token.unwrap_or(var("VAULT_TOKEN").unwrap());
        read_secret_key(settings, &vault_token, ETH_VAULT_PREFIX, "key").await
    }
}

#[cfg(test)]
pub mod tests {
    use crate::vault_private_key::{EthKey, NearKey};

    #[tokio::test]
    async fn read_vault_eth_key() {
        let settings = crate::test_utils::get_settings();
        dotenv::dotenv().ok();
        let vault_token = std::env::var("VAULT_TOKEN").unwrap();

        let private_key = EthKey::vault_private_key(&settings, Some(vault_token)).await;
        assert_eq!(private_key.len(), 64);
    }

    #[tokio::test]
    async fn read_vault_near_key() {
        let settings = crate::test_utils::get_settings();
        dotenv::dotenv().ok();
        let vault_token = std::env::var("VAULT_TOKEN").unwrap();

        let private_key = NearKey::vault_private_key(&settings, &vault_token).await;
        assert_eq!(private_key.len(), 95);
    }

    #[tokio::test]
    async fn read_vault_near_account() {
        let settings = crate::test_utils::get_settings();
        dotenv::dotenv().ok();
        let vault_token = std::env::var("VAULT_TOKEN").unwrap();

        let account_id = NearKey::vault_account_id(&settings, &vault_token).await;
        assert_eq!(account_id, "olga24912.testnet");
    }
}
