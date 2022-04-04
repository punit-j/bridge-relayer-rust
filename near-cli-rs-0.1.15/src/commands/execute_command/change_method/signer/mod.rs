#[derive(Debug, Clone)]
pub struct Sender {
    pub sender_account_id: crate::types::account_id::AccountId,
    pub sign_option:
        crate::commands::construct_transaction_command::sign_transaction::SignTransaction,
}

impl Sender {
    // fn from_cli_sender_account_id(
    //     optional_cli_sender_account_id: Option<crate::types::account_id::AccountId>,
    //     context: &super::operation_mode::ExecuteChangeMethodCommandNetworkContext,
    // ) -> color_eyre::eyre::Result<crate::types::account_id::AccountId> {
    //     match optional_cli_sender_account_id {
    //         Some(cli_sender_account_id) => match &context.connection_config {
    //             Some(network_connection_config) => match crate::common::get_account_state(
    //                 &network_connection_config,
    //                 cli_sender_account_id.clone().into(),
    //             )? {
    //                 Some(_) => Ok(cli_sender_account_id),
    //                 None => {
    //                     println!("Account <{}> doesn't exist", cli_sender_account_id);
    //                     Sender::input_sender_account_id(&context)
    //                 }
    //             },
    //             None => Ok(cli_sender_account_id),
    //         },
    //         None => Self::input_sender_account_id(&context),
    //     }
    // }

    // fn input_sender_account_id(
    //     context: &super::operation_mode::ExecuteChangeMethodCommandNetworkContext,
    // ) -> color_eyre::eyre::Result<crate::types::account_id::AccountId> {
    //     loop {
    //         let account_id: crate::types::account_id::AccountId = Input::new()
    //             .with_prompt("What is the account ID of the sender?")
    //             .interact_text()?;
    //         if let Some(connection_config) = &context.connection_config {
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
        match self
            .sign_option
            .process(unsigned_transaction, network_connection_config.clone())
            .await?
        {
            Some(transaction_info) => {
                crate::common::print_transaction_status(
                    transaction_info,
                    network_connection_config,
                );
            }
            None => {}
        };
        Ok(())
    }
}
