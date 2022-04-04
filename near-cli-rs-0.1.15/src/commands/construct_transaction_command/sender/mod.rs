#[derive(Debug, Clone)]
pub struct Sender {
    pub sender_account_id: crate::types::account_id::AccountId,
    pub receiver: super::receiver::Receiver,
}

impl Sender {
    // fn input_sender_account_id(
    //     context: &super::operation_mode::ConstructTransactionNetworkContext,
    // ) -> color_eyre::eyre::Result<crate::types::account_id::AccountId> {
    //     let connection_config = context.connection_config.clone();
    //     loop {
    //         let account_id: crate::types::account_id::AccountId = Input::new()
    //             .with_prompt("What is the account ID of the sender?")
    //             .interact_text()?;
    //         if let Some(connection_config) = &connection_config {
    //             if let Some(_) =
    //                 crate::common::get_account_state(&connection_config, account_id.clone().into())?
    //             {
    //                 break Ok(account_id);
    //             } else {
    //                 println!("Account <{}> doesn't exist", account_id.to_string());
    //             }
    //         } else {
    //             break Ok(account_id);
    //         }
    //     }
    // }

    pub async fn process(
        self,
        prepopulated_unsigned_transaction: near_primitives::transaction::Transaction,
        network_connection_config: Option<crate::common::ConnectionConfig>,
    ) -> crate::CliResult {
        let unsigned_transaction = near_primitives::transaction::Transaction {
            signer_id: self.sender_account_id.clone().into(),
            ..prepopulated_unsigned_transaction
        };
        self.receiver
            .process(unsigned_transaction, network_connection_config)
            .await
    }
}
