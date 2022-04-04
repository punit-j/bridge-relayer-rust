#[derive(Debug, Clone)]
pub struct Contract {
    pub contract_account_id: crate::types::account_id::AccountId,
    pub call: super::call_function_type::CallFunctionAction,
}

impl Contract {
    // fn input_contract_account_id(
    //     context: &super::operation_mode::ExecuteChangeMethodCommandNetworkContext,
    // ) -> color_eyre::eyre::Result<crate::types::account_id::AccountId> {
    //     let connection_config = context.connection_config.clone();
    //     loop {
    //         let account_id: crate::types::account_id::AccountId = Input::new()
    //             .with_prompt("What is the account ID of the contract?")
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
            receiver_id: self.contract_account_id.clone().into(),
            ..prepopulated_unsigned_transaction
        };
        self.call
            .process(unsigned_transaction, network_connection_config)
            .await
    }
}
