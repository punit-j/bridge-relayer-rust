use crate::read_private_key::read_private_key_from_file;
use near_crypto::InMemorySigner;
use near_sdk::AccountId;
use std::ffi::OsStr;
use std::path::Path;

pub const NEAR_TOKEN_ADDRESS: &str = "token.olga24912_3.testnet";

pub fn abspath(p: &str) -> Option<String> {
    shellexpand::full(p)
        .ok()
        .and_then(|x| Path::new(OsStr::new(x.as_ref())).canonicalize().ok())
        .and_then(|p| p.into_os_string().into_string().ok())
}

pub fn get_near_signer() -> InMemorySigner {
    let path = "~/.near-credentials/testnet/spectrebridge.testnet.json";
    let absolute = abspath(path).unwrap();
    read_private_key_from_file(&absolute).unwrap()
}

pub fn get_near_token() -> AccountId {
    NEAR_TOKEN_ADDRESS.parse().unwrap()
}

pub fn get_server_addr() -> url::Url {
    url::Url::parse("https://rpc.testnet.near.org").unwrap()
}