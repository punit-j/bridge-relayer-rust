mod offline_mode;
mod online_mode;

#[derive(Debug, Clone)]
pub struct OperationMode {
    pub mode: Mode,
}

impl OperationMode {
    pub async fn process(
        self,
        prepopulated_unsigned_transaction: near_primitives::transaction::Transaction,
    ) -> crate::CliResult {
        self.mode.process(prepopulated_unsigned_transaction).await
    }
}

#[derive(Debug, Clone)]
///To construct a transaction you will need to provide information about sender (signer) and receiver accounts, and actions that needs to be performed.
///Do you want to derive some information required for transaction construction automatically querying it online?
pub enum Mode {
    Network(self::online_mode::NetworkArgs),
    Offline(self::offline_mode::OfflineArgs),
}

impl Mode {
    pub async fn process(
        self,
        prepopulated_unsigned_transaction: near_primitives::transaction::Transaction,
    ) -> crate::CliResult {
        match self {
            Self::Network(network_args) => {
                network_args
                    .process(prepopulated_unsigned_transaction)
                    .await
            }
            Self::Offline(offline_args) => {
                offline_args
                    .process(prepopulated_unsigned_transaction)
                    .await
            }
        }
    }
}

pub struct ExecuteChangeMethodCommandNetworkContext {
    pub connection_config: Option<crate::common::ConnectionConfig>,
}
