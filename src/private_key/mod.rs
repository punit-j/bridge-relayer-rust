use crate::Settings;
use serde_json::{json, Value};
use std::fs;
use std::ops::Deref;
use std::path::Path;

use http_client::HttpClient;
use http_types::{Method, Request};

const NEAR_VAULT_PREFIX: &str = "key2"; // Change names according to production
const ETH_VAULT_PREFIX: &str = "key3";

pub struct NearKey {}

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

impl NearKey {
    pub fn local_private_key(file_path: Box<&Path>) -> String {
        let credential_data =
            fs::read_to_string(file_path.deref().to_str().unwrap()).expect("Unable to read file");
        let json: serde_json::Value = serde_json::from_str(&credential_data).unwrap();

        let key: String = json
            .get("private_key")
            .expect("Cannot get NEAR private key from given credential")
            .to_string();

        key.to_string().replace(&['\"'], "")
    }

    pub async fn vault_private_key(settings: &Settings, vault_token: &String) -> String {
        read_secret_key(settings, vault_token, NEAR_VAULT_PREFIX).await
    }
}

pub struct EthKey {}

impl EthKey {
    pub fn local_private_key(file_path: Box<&Path>) -> String {
        fs::read_to_string(file_path.deref().to_str().unwrap()).expect("Unable to read file")
    }

    pub async fn vault_private_key(settings: &Settings, vault_token: &String) -> String {
        read_secret_key(settings, vault_token, ETH_VAULT_PREFIX).await
    }
}
