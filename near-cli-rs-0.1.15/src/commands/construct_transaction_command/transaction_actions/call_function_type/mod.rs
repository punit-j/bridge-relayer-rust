use async_recursion::async_recursion;
use dialoguer::Input;

#[derive(Debug, Clone)]
pub struct CallFunctionAction {
    method_name: String,
    args: Vec<u8>,
    gas: near_primitives::types::Gas,
    deposit: near_primitives::types::Balance,
    next_action: Box<super::NextAction>,
}

impl CallFunctionAction {
    // fn input_method_name(
    //     _context: &crate::common::SignerContext,
    // ) -> color_eyre::eyre::Result<String> {
    //     println!();
    //     Ok(Input::new()
    //         .with_prompt("Enter a method name")
    //         .interact_text()?)
    // }

    // fn input_gas(
    //     _context: &crate::common::SignerContext,
    // ) -> color_eyre::eyre::Result<near_primitives::types::Gas> {
    //     println!();
    //     let gas: u64 = loop {
    //         let input_gas: crate::common::NearGas = Input::new()
    //             .with_prompt("Enter a gas for function")
    //             .with_initial_text("100 TeraGas")
    //             .interact_text()?;
    //         let gas: u64 = match input_gas {
    //             crate::common::NearGas { inner: num } => num,
    //         };
    //         if gas <= 300000000000000 {
    //             break gas;
    //         } else {
    //             println!("You need to enter a value of no more than 300 TERAGAS")
    //         }
    //     };
    //     Ok(gas)
    // }

    // fn input_args(_context: &crate::common::SignerContext) -> color_eyre::eyre::Result<Vec<u8>> {
    //     println!();
    //     let input: String = Input::new()
    //         .with_prompt("Enter args for function")
    //         .interact_text()?;
    //     Ok(input.into_bytes())
    // }

    // fn input_deposit(
    //     _context: &crate::common::SignerContext,
    // ) -> color_eyre::eyre::Result<near_primitives::types::Balance> {
    //     println!();
    //     let deposit: crate::common::NearBalance = Input::new()
    //         .with_prompt(
    //             "Enter a deposit for function (example: 10NEAR or 0.5near or 10000yoctonear).",
    //         )
    //         .with_initial_text("0 NEAR")
    //         .interact_text()?;
    //     Ok(deposit.to_yoctonear())
    // }

    #[async_recursion(?Send)]
    pub async fn process(
        self,
        prepopulated_unsigned_transaction: near_primitives::transaction::Transaction,
        network_connection_config: Option<crate::common::ConnectionConfig>,
    ) -> crate::CliResult {
        let action = near_primitives::transaction::Action::FunctionCall(
            near_primitives::transaction::FunctionCallAction {
                method_name: self.method_name.clone(),
                args: self.args.clone(),
                gas: self.gas.clone(),
                deposit: self.deposit.clone(),
            },
        );
        let mut actions = prepopulated_unsigned_transaction.actions.clone();
        actions.push(action);
        let unsigned_transaction = near_primitives::transaction::Transaction {
            actions,
            ..prepopulated_unsigned_transaction
        };
        match *self.next_action {
            super::NextAction::AddAction(select_action) => {
                select_action
                    .process(unsigned_transaction, network_connection_config)
                    .await
            }
            super::NextAction::Skip(skip_action) => {
                skip_action
                    .process(unsigned_transaction, network_connection_config)
                    .await
            }
        }
    }
}
