use crate::config::Settings;
use serde_json::json;
use std::fs;
use std::ops::Deref;
use std::path::Path;

use http_client::HttpClient;
use http_types::{Method, Request};

#[allow(unused)]
const NEAR_VAULT_PREFIX: &str = "key2"; // Change names according to production
#[allow(unused)]
const ETH_VAULT_PREFIX: &str = "key3";

#[allow(unused)]
pub struct NearKey {}

#[allow(unused)]
async fn read_secret_key(settings: &Settings, vault_token: &String, prefix: &str) -> String {
    let client = http_client::h1::H1Client::new();
    let mut addr = settings.vault_addr.to_string();
    addr.push_str(prefix);

    let mut request = Request::new(Method::Get, addr.as_str());
    request.insert_header("X-Vault-Token", vault_token);

    let res = json!(client
        .send(request)
        .await
        .unwrap()
        .body_string()
        .await
        .unwrap());

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
            .get("key")
            .expect("Cannot read private key from Vault response")
            .to_string()
            .replace(&['\"'], ""),
    }
}

#[allow(unused)]
impl NearKey {
    pub fn local_private_key(file_path: &Path) -> String {
        let credential_data =
            fs::read_to_string(file_path.deref().to_str().unwrap()).expect("Unable to read file");
        let json: serde_json::Value = serde_json::from_str(&credential_data).unwrap();

        let key: String = json
            .get("private_key")
            .expect("Cannot get NEAR private key from given credential")
            .to_string();

        key.replace(&['\"'], "")
    }

    pub async fn vault_private_key(settings: &Settings, vault_token: &String) -> String {
        read_secret_key(settings, vault_token, NEAR_VAULT_PREFIX).await
    }
}

#[allow(unused)]
pub struct EthKey {}

#[allow(unused)]
impl EthKey {
    pub fn local_private_key(file_path: &Path) -> String {
        fs::read_to_string(file_path.deref().to_str().unwrap()).expect("Unable to read file")
    }

    pub async fn vault_private_key(settings: &Settings, vault_token: &String) -> String {
        read_secret_key(settings, vault_token, ETH_VAULT_PREFIX).await
    }
}
