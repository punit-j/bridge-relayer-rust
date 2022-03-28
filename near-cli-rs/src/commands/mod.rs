use strum::{EnumDiscriminants, EnumIter, EnumMessage};

pub mod construct_transaction_command;
pub mod execute_command;

#[derive(Debug, Clone, EnumDiscriminants, interactive_clap_derive::InteractiveClap)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
#[interactive_clap(context = ())]
///Choose transaction action
pub enum TopLevelCommand {
    #[strum_discriminants(strum(message = "Execute function (contract method)"))]
    ///Execute function (contract method)
    Execute(self::execute_command::OptionMethod),
}

impl TopLevelCommand {
    pub async fn process(self) -> crate::CliResult {
        let unsigned_transaction = near_primitives::transaction::Transaction {
            signer_id: "test".parse().unwrap(),
            public_key: near_crypto::PublicKey::empty(near_crypto::KeyType::ED25519),
            nonce: 0,
            receiver_id: "test".parse().unwrap(),
            block_hash: Default::default(),
            actions: vec![],
        };
        match self {
            Self::Execute(option_method) => option_method.process(unsigned_transaction).await,
        }
    }
}
