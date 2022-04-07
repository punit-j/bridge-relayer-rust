
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Transfer {
    pub token: web3::types::Address,
    //web3::types::Address,
    pub amount: u128
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SpectreBridgeTransferEvent {
    pub valid_till: u64,
    // unix_timestamp when transaction is expired,
    pub transfer: Transfer,
    // token account on ethereum side and eth amount
    pub fee: Transfer,
    // AccountId of token in which fee is paid and amount of fee paid to LP-Relayer for transferring
    pub recipient: web3::types::Address // recipient on Ethereum side
}

#[cfg(test)]
pub mod tests {
    use std::str::FromStr;

    pub fn test_struct_build() -> super::SpectreBridgeTransferEvent {
        super::SpectreBridgeTransferEvent {
            valid_till: 123897,
            transfer: super::Transfer {
                token: web3::types::Address::from_str("0xd034739c2ae107c70cd703092b946f12a49509d1").unwrap(),
                amount: 85 },
            fee: super::Transfer {
                token: web3::types::Address::from_str("0xd034739c2ae807c70cd703092b947f72a49509d1").unwrap(),
                amount: 789
            },
            recipient: web3::types::Address::from_str("0xd034739c2ae807c70cd743492b946f62a49509d1").unwrap()
        }
    }

    pub fn test_struct_check(first: &super::SpectreBridgeTransferEvent, second: &super::SpectreBridgeTransferEvent) {
        assert_eq!(first.valid_till, second.valid_till);
        assert_eq!(first.transfer, second.transfer);
        assert_eq!(first.fee, second.fee);
        assert_eq!(first.recipient, second.recipient);
    }

    #[test]
    fn serialize() {
        let tt = test_struct_build();

        let serialize = serde_json::to_string(&tt).unwrap();
        let res: super::SpectreBridgeTransferEvent = serde_json::from_str(&serialize).unwrap();

        test_struct_check(&tt, &res);
    }
}