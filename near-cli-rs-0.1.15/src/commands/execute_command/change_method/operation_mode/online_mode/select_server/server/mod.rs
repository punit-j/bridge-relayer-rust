#[derive(Debug, Clone)]
pub struct Server {
    pub contract: super::super::super::super::contract::Contract,
}

#[derive(Debug, Clone)]
pub struct CustomServer {
    pub url: crate::common::AvailableRpcServerUrl,
    pub contract: super::super::super::super::contract::Contract,
}

impl Server {
    pub async fn process(
        self,
        prepopulated_unsigned_transaction: near_primitives::transaction::Transaction,
        connection_config: crate::common::ConnectionConfig,
    ) -> crate::CliResult {
        self.contract
            .process(prepopulated_unsigned_transaction, Some(connection_config))
            .await
    }
}

impl CustomServer {
    pub async fn process(
        self,
        prepopulated_unsigned_transaction: near_primitives::transaction::Transaction,
    ) -> crate::CliResult {
        let connection_config = Some(crate::common::ConnectionConfig::from_custom_url(&self.url));
        self.contract
            .process(prepopulated_unsigned_transaction, connection_config)
            .await
    }
}
