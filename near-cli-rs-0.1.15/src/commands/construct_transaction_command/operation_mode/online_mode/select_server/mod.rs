use strum::{EnumDiscriminants, EnumIter, EnumMessage};

pub mod server;

#[derive(Debug, Clone, EnumDiscriminants)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
///Select NEAR protocol RPC server
pub enum SelectServer {
    Testnet(self::server::Server),
    Mainnet(self::server::Server),
    Betanet(self::server::Server),
    Custom(self::server::CustomServer),
}

impl SelectServer {
    pub async fn process(
        self,
        prepopulated_unsigned_transaction: near_primitives::transaction::Transaction,
    ) -> crate::CliResult {
        Ok(match self {
            SelectServer::Testnet(server) => {
                let connection_config = crate::common::ConnectionConfig::Testnet;
                server
                    .process(prepopulated_unsigned_transaction, connection_config)
                    .await?;
            }
            SelectServer::Mainnet(server) => {
                let connection_config = crate::common::ConnectionConfig::Mainnet;
                server
                    .process(prepopulated_unsigned_transaction, connection_config)
                    .await?;
            }
            SelectServer::Betanet(server) => {
                let connection_config = crate::common::ConnectionConfig::Betanet;
                server
                    .process(prepopulated_unsigned_transaction, connection_config)
                    .await?;
            }
            SelectServer::Custom(custom_server) => {
                custom_server
                    .process(prepopulated_unsigned_transaction)
                    .await?;
            }
        })
    }
}
