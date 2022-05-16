use near_sdk::{log, serde::Deserialize, serde::Serialize, serde_json, AccountId};
use serde_json::json;

pub type EthAddress = [u8; 20];

pub const STANDARD: &str = "nep297";
pub const VERSION: &str = "1.0.0";

#[derive(Serialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct TransferDataEthereum {
    token: EthAddress,
    amount: u128,
}

#[derive(Serialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct TransferDataNear {
    pub(crate) token: AccountId,
    pub(crate) amount: u128,
}

#[derive(Serialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
#[allow(dead_code)]
pub enum Event<'a> {
    SpectreBridgeNonceEvent {
        nonce: &'a u128,
        account: &'a AccountId,
        transfer: &'a TransferDataEthereum,
        recipient: &'a EthAddress,
    },
    SpectreBridgeTransferEvent {
        nonce: &'a u128,
        valid_till: u64,
        transfer: &'a TransferDataNear,
        fee: &'a TransferDataNear,
        recipient: &'a EthAddress,
    },
    SpectreBridgeTransferFailedEvent {
        nonce: &'a u128,
        account: &'a AccountId,
    },
    SpectreBridgeUnlockEvent {
        nonce: &'a u128,
        account: &'a AccountId,
    },
    SpectreBridgeDepositEvent {
        account: &'a AccountId,
        token: &'a AccountId,
        amount: &'a u128,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct EventMessage {
    pub standard: String,
    pub version: String,
    pub event: serde_json::Value,
    pub data: [serde_json::Value; 1],
}
