#[derive(Debug, Clone)]
pub struct CallFunctionView {
    pub method_name: String,
    pub function_args: String,
    pub selected_block_id: super::block_id::BlockId,
}

impl CallFunctionView {
    pub async fn process(
        self,
        network_connection_config: crate::common::ConnectionConfig,
        contract_account_id: near_primitives::types::AccountId,
    ) -> crate::CliResult {
        self.selected_block_id
            .process(
                contract_account_id,
                network_connection_config,
                self.method_name,
                self.function_args.into_bytes(),
            )
            .await
    }
}
