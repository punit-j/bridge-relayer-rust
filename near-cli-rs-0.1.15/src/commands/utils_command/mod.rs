use strum::{EnumDiscriminants, EnumIter, EnumMessage, IntoEnumIterator};

mod combine_transaction_subcommand_with_signature;
pub mod generate_keypair_subcommand;
mod send_signed_transaction;
mod sign_transaction_subcommand_with_secret_key;
mod view_serialized_transaction;

#[derive(Debug, Clone)]
pub struct Utils {
    pub util: Util,
}

impl Utils {
    pub async fn process(self) -> crate::CliResult {
        self.util.process().await
    }
}

#[derive(Debug, Clone, EnumDiscriminants)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
pub enum Util {
    GenerateKeypair(self::generate_keypair_subcommand::CliGenerateKeypair),
    SignTransactionPrivateKey(
        self::sign_transaction_subcommand_with_secret_key::SignTransactionPrivateKey,
    ),
    CombineTransactionSignature(
        self::combine_transaction_subcommand_with_signature::CombineTransactionSignature,
    ),
    ViewSerializedTransaction(self::view_serialized_transaction::ViewSerializedTransaction),
    SendSignedTransaction(self::send_signed_transaction::operation_mode::OperationMode),

}

impl Util {
    pub async fn process(self) -> crate::CliResult {
        match self {
            Self::GenerateKeypair(generate_keypair) => generate_keypair.process().await,
            Self::SignTransactionPrivateKey(sign_transaction) => sign_transaction.process().await,
            Self::CombineTransactionSignature(combine_transaction) => {
                combine_transaction.process().await
            }
            Self::ViewSerializedTransaction(view_serialized_transaction) => {
                view_serialized_transaction.process().await
            }
            Self::SendSignedTransaction(operation_mode) => operation_mode.process().await,
        }
    }
}
