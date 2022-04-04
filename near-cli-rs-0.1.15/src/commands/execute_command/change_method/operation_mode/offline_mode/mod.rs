#[derive(Debug, Clone)]
pub struct OfflineArgs {
    contract: super::super::contract::Contract,
}

impl OfflineArgs {
    pub async fn process(
        self,
        prepopulated_unsigned_transaction: near_primitives::transaction::Transaction,
    ) -> crate::CliResult {
        let selected_server_url = None;
        self.contract
            .process(prepopulated_unsigned_transaction, selected_server_url)
            .await
    }
}
