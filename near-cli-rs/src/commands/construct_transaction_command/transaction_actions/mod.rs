use strum::{EnumDiscriminants, EnumIter, EnumMessage};

mod call_function_type;

#[derive(Debug, Clone, clap::Clap)]
pub enum CliSkipNextAction {
    /// Go to transaction signing
    Skip(CliSkipAction),
}

#[derive(Debug, Clone, EnumDiscriminants, interactive_clap_derive::InteractiveClap)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
#[interactive_clap(context = crate::common::SignerContext)]
///Select an action that you want to add to the action:
pub enum NextAction {
    #[strum_discriminants(strum(message = "Select a new action"))]
    /// Choose next action
    AddAction(SelectAction),
    #[strum_discriminants(strum(message = "Skip adding a new action"))]
    /// Go to transaction signing
    Skip(SkipAction),
}

impl CliSkipNextAction {
    pub fn to_cli_args(&self) -> std::collections::VecDeque<String> {
        match self {
            Self::Skip(subcommand) => {
                // let mut args = ;  it is not implemented now!!!
                // args.push_front("skip".to_owned());
                // args
                subcommand.to_cli_args()
            }
        }
    }
}

impl From<NextAction> for CliSkipNextAction {
    fn from(next_action: NextAction) -> Self {
        match next_action {
            NextAction::AddAction(_select_action) => {
                Self::Skip(CliSkipAction { sign_option: None })
            }
            NextAction::Skip(skip_action) => Self::Skip(skip_action.into()),
        }
    }
}

impl NextAction {
    pub fn from_cli_skip_next_action(
        item: CliSkipNextAction,
        context: crate::common::SignerContext,
    ) -> color_eyre::eyre::Result<Self> {
        match item {
            CliSkipNextAction::Skip(cli_skip_action) => {
                let skip_action: SkipAction = SkipAction::from_cli(Some(cli_skip_action), context)?;
                Ok(Self::Skip(skip_action))
            }
        }
    }
}
//-------------------------------------
/// This mode is not provided now
// impl CliNextAction {
//     fn from(item: CliSkipNextAction) -> color_eyre::eyre::Result<Self> {
//         match item {
//             CliSkipNextAction::Skip(cli_skip_action) => Ok(Self::Skip(cli_skip_action)),
//         }
//     }
// }
//--------------------------------------
impl NextAction {
    pub async fn process(
        self,
        prepopulated_unsigned_transaction: near_primitives::transaction::Transaction,
        network_connection_config: Option<crate::common::ConnectionConfig>,
    ) -> crate::CliResult {
        match self {
            NextAction::AddAction(select_action) => {
                select_action
                    .process(prepopulated_unsigned_transaction, network_connection_config)
                    .await
            }
            NextAction::Skip(skip_action) => {
                skip_action
                    .process(prepopulated_unsigned_transaction, network_connection_config)
                    .await
            }
        }
    }
}

#[derive(Debug, Clone, interactive_clap_derive::InteractiveClap)]
#[interactive_clap(context = crate::common::SignerContext)]
pub struct SelectAction {
    #[interactive_clap(subcommand)]
    transaction_subcommand: ActionSubcommand,
}

impl SelectAction {
    pub async fn process(
        self,
        prepopulated_unsigned_transaction: near_primitives::transaction::Transaction,
        network_connection_config: Option<crate::common::ConnectionConfig>,
    ) -> crate::CliResult {
        self.transaction_subcommand
            .process(prepopulated_unsigned_transaction, network_connection_config)
            .await
    }
}

#[derive(Debug, Clone, EnumDiscriminants, interactive_clap_derive::InteractiveClap)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
#[interactive_clap(context = crate::common::SignerContext)]
///Select an action that you want to add to the action:
pub enum ActionSubcommand {
    #[strum_discriminants(strum(message = "Call a Function"))]
    /// Provide data for calling a function
    CallFunction(self::call_function_type::CallFunctionAction),
}

impl ActionSubcommand {
    pub async fn process(
        self,
        prepopulated_unsigned_transaction: near_primitives::transaction::Transaction,
        network_connection_config: Option<crate::common::ConnectionConfig>,
    ) -> crate::CliResult {
        match self {
            ActionSubcommand::CallFunction(args_function) => {
                args_function
                    .process(prepopulated_unsigned_transaction, network_connection_config)
                    .await
            }
        }
    }
}

#[derive(Debug, Clone, interactive_clap_derive::InteractiveClap)]
#[interactive_clap(context = crate::common::SignerContext)]
pub struct SkipAction {
    #[interactive_clap(subcommand)]
    pub sign_option: super::sign_transaction::SignTransaction,
}
//------------------------------------
// impl From<SelectAction> for CliSkipAction {
//     fn from(select_action: SelectAction) -> Self {
//         Self{
//             sign_option:
//         }
//     }
// }
//-----------------------------------------

impl SkipAction {
    pub async fn process(
        self,
        prepopulated_unsigned_transaction: near_primitives::transaction::Transaction,
        network_connection_config: Option<crate::common::ConnectionConfig>,
    ) -> crate::CliResult {
        match self
            .sign_option
            .process(
                prepopulated_unsigned_transaction,
                network_connection_config.clone(),
            )
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
