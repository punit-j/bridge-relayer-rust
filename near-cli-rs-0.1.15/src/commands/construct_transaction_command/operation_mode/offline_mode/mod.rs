#[derive(Debug, Clone)]
pub struct OfflineArgs {
    sender: super::super::sender::Sender,
}

impl OfflineArgs {
    pub async fn process(
        self,
        prepopulated_unsigned_transaction: near_primitives::transaction::Transaction,
    ) -> crate::CliResult {
        let selected_server_url = None;
        self.sender
            .process(prepopulated_unsigned_transaction, selected_server_url)
            .await
    }
}
