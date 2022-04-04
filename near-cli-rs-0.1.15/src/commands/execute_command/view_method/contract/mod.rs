#[derive(Debug, Clone)]
pub struct Contract {
    pub contract_account_id: crate::types::account_id::AccountId,
    pub call: super::call_function_type::CallFunctionView,
}

impl Contract {
    // fn input_contract_account_id(
    //     context: &super::operation_mode::online_mode::select_server::ExecuteViewMethodCommandNetworkContext,
    // ) -> color_eyre::eyre::Result<crate::types::account_id::AccountId> {
    //     let connection_config = context.connection_config.clone();
    //     loop {
    //         let contract_account_id: crate::types::account_id::AccountId = Input::new()
    //             .with_prompt("What is the account ID of the contract?")
    //             .interact_text()?;
    //         let contract_code_hash: near_primitives::hash::CryptoHash =
    //             match crate::common::get_account_state(
    //                 &connection_config,
    //                 contract_account_id.clone().into(),
    //             )? {
    //                 Some(account_view) => account_view.code_hash,
    //                 None => near_primitives::hash::CryptoHash::default(),
    //             };
    //         if contract_code_hash == near_primitives::hash::CryptoHash::default() {
    //             println!(
    //                 "Contract code is not deployed to this account <{}>.",
    //                 contract_account_id.to_string()
    //             )
    //         } else {
    //             break Ok(contract_account_id);
    //         }
    //     }
    // }

    pub async fn process(
        self,
        network_connection_config: crate::common::ConnectionConfig,
    ) -> crate::CliResult {
        self.call
            .process(network_connection_config, self.contract_account_id.into())
            .await
    }
}
