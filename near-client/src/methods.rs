use std::io;

use serde::Deserialize;
use serde_json::json;
use thiserror::Error;

mod private {
    pub trait Sealed {}
}

pub trait RpcMethod: private::Sealed
where
    Self::Response: RpcHandlerResponse,
    Self::Error: RpcHandlerError,
{
    type Response;
    type Error;

    fn method_name(&self) -> &str;

    fn params(&self) -> Result<serde_json::Value, io::Error>;
}

impl<T> private::Sealed for &T where T: private::Sealed {}
impl<T> RpcMethod for &T
where
    T: RpcMethod,
{
    type Response = T::Response;
    type Error = T::Error;

    fn method_name(&self) -> &str {
        T::method_name(self)
    }

    fn params(&self) -> Result<serde_json::Value, io::Error> {
        T::params(self)
    }
}

pub trait RpcHandlerResponse: serde::de::DeserializeOwned {
    fn parse(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }
}

pub trait RpcHandlerError: serde::de::DeserializeOwned {
    /// Parser for the `.error_struct` field in RpcError.
    fn parse(handler_error: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(handler_error)
    }

    /// Parser for the `.data` field in RpcError, not `.error_struct`.
    ///
    /// This would only ever be used as a fallback if [`RpcHandlerError::parse`] fails.
    ///
    /// Defaults to `None` meaning there's no alternative deserialization available.
    fn parse_raw_error(_error: serde_json::Value) -> Option<Result<Self, serde_json::Error>> {
        None
    }
}

macro_rules! impl_method {
    (
        $(#[$meta:meta])*
        pub mod $method_name:ident {
            $($body:tt)+
        }
    ) => {
        #[allow(non_snake_case)]
        pub mod $method_name {
            $(#![$meta])*

            use super::*;

            const METHOD_NAME: &'static str = stringify!($method_name);

            $($body)+
        }
    }
}

macro_rules! impl_ {
    ($valid_trait:ident for $for_type:ty { $($body:tt)+ }) => {
        impl private::Sealed for $for_type {}
        impl $valid_trait for $for_type {
            $($body)+

            #[inline(always)]
            fn method_name(&self) -> &str {
                METHOD_NAME
            }
        }
    };
}

// workaround for deserializing partially serialized
// error types missing the `error_message` field in
// their UnknownBlock variants.
macro_rules! parse_unknown_block {
    ($json:expr => $err_ty:ident) => {
        if $json["name"] == "UNKNOWN_BLOCK" {
            Some(Ok($err_ty::UnknownBlock {
                error_message: "".to_string(),
            }))
        } else {
            None
        }
    };
}

mod shared_impls {
    use super::{RpcHandlerError, RpcHandlerResponse};

    // adv_*
    #[cfg(feature = "adversarial")]
    impl RpcHandlerError for () {}

    // adv_*
    #[cfg(feature = "adversarial")]
    impl RpcHandlerResponse for () {
        fn parse(_value: serde_json::Value) -> Result<Self, serde_json::Error> {
            Ok(())
        }
    }

    #[cfg(feature = "any")]
    impl RpcHandlerResponse for serde_json::Value {
        fn parse(value: serde_json::Value) -> Result<Self, serde_json::Error> {
            Ok(value)
        }
    }

    #[cfg(feature = "any")]
    impl RpcHandlerError for serde_json::Value {
        fn parse(handler_error: serde_json::Value) -> Result<Self, serde_json::Error> {
            Ok(handler_error)
        }
    }

    // broadcast_tx_commit, tx
    impl RpcHandlerResponse for near_primitives::views::FinalExecutionOutcomeView {}

    // broadcast_tx_commit, tx, EXPERIMENTAL_check_tx, EXPERIMENTAL_tx_status
    impl RpcHandlerError for near_jsonrpc_primitives::types::transactions::RpcTransactionError {
        fn parse_raw_error(value: serde_json::Value) -> Option<Result<Self, serde_json::Error>> {
            match serde_json::from_value::<near_jsonrpc_primitives::errors::ServerError>(value) {
                Ok(near_jsonrpc_primitives::errors::ServerError::TxExecutionError(
                    near_primitives::errors::TxExecutionError::InvalidTxError(context),
                )) => Some(Ok(Self::InvalidTransaction { context })),
                Err(err) => Some(Err(err)),
                _ => None,
            }
        }
    }

    // health, status
    impl RpcHandlerError for near_jsonrpc_primitives::types::status::RpcStatusError {}

    // EXPERIMENTAL_changes, EXPERIMENTAL_changes_in_block
    impl RpcHandlerError for near_jsonrpc_primitives::types::changes::RpcStateChangesError {
        fn parse_raw_error(value: serde_json::Value) -> Option<Result<Self, serde_json::Error>> {
            parse_unknown_block!(value => Self)
        }
    }

    // EXPERIMENTAL_broadcast_tx_sync, EXPERIMENTAL_check_tx
    impl RpcHandlerResponse
        for near_jsonrpc_primitives::types::transactions::RpcBroadcastTxSyncResponse
    {
    }

    // validators, EXPERIMENTAL_validators_ordered
    impl RpcHandlerError for near_jsonrpc_primitives::types::validator::RpcValidatorError {}
}

#[cfg(feature = "any")]
pub use any::{request as any, RpcAnyRequest};

#[cfg(feature = "any")]
mod any {
    use super::*;
    use std::marker::PhantomData;

    pub fn request<T: AnyRequestResult>(
        method_name: &str,
        params: serde_json::Value,
    ) -> RpcAnyRequest<T::Response, T::Error>
    where
        T::Response: RpcHandlerResponse,
        T::Error: RpcHandlerError,
    {
        RpcAnyRequest {
            method: method_name.to_string(),
            params,
            _data: PhantomData,
        }
    }

    #[derive(Debug)]
    pub struct RpcAnyRequest<T, E> {
        pub method: String,
        pub params: serde_json::Value,
        pub(crate) _data: PhantomData<(T, E)>,
    }

    impl<T, E> private::Sealed for RpcAnyRequest<T, E> {}

    impl<T, E> RpcMethod for RpcAnyRequest<T, E>
    where
        T: RpcHandlerResponse,
        E: RpcHandlerError,
    {
        type Response = T;
        type Error = E;

        #[inline(always)]
        fn method_name(&self) -> &str {
            &self.method
        }

        fn params(&self) -> Result<serde_json::Value, io::Error> {
            Ok(self.params.clone())
        }
    }

    pub trait AnyRequestResult {
        type Response;
        type Error;
    }

    impl<T, E> AnyRequestResult for Result<T, E> {
        type Response = T;
        type Error = E;
    }

    impl<T: RpcMethod> AnyRequestResult for T {
        type Response = T::Response;
        type Error = T::Error;
    }
}

impl_method! {
    pub mod block {
        pub use near_jsonrpc_primitives::types::blocks::RpcBlockError;
        pub use near_jsonrpc_primitives::types::blocks::RpcBlockRequest;

        pub type RpcBlockResponse = near_primitives::views::BlockView;

        impl RpcHandlerResponse for RpcBlockResponse {}

        impl RpcHandlerError for RpcBlockError {
            fn parse_raw_error(value: serde_json::Value) -> Option<Result<Self, serde_json::Error>> {
                parse_unknown_block!(value => Self)
            }
        }

        impl_!(RpcMethod for RpcBlockRequest {
            type Response = RpcBlockResponse;
            type Error = RpcBlockError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(self))
            }
        });
    }
}

fn serialize_signed_transaction(
    tx: &near_primitives::transaction::SignedTransaction,
) -> Result<String, io::Error> {
    Ok(near_primitives::serialize::to_base64(
        &borsh::BorshSerialize::try_to_vec(&tx)?,
    ))
}

impl_method! {
    pub mod broadcast_tx_async {
        pub use near_primitives::transaction::SignedTransaction;

        pub type RpcBroadcastTxAsyncResponse = near_primitives::hash::CryptoHash;

        #[derive(Debug)]
        pub struct RpcBroadcastTxAsyncRequest {
            pub signed_transaction: SignedTransaction,
        }

        impl From<RpcBroadcastTxAsyncRequest>
            for near_jsonrpc_primitives::types::transactions::RpcBroadcastTransactionRequest
        {
            fn from(this: RpcBroadcastTxAsyncRequest) -> Self {
                Self {
                    signed_transaction: this.signed_transaction,
                }
            }
        }

        #[derive(Debug, Deserialize, Error)]
        #[error("{}", unreachable!("fatal: this error should never be constructed"))]
        pub enum RpcBroadcastTxAsyncError {}

        impl RpcHandlerResponse for RpcBroadcastTxAsyncResponse {}

        impl RpcHandlerError for RpcBroadcastTxAsyncError {}

        impl_!(RpcMethod for RpcBroadcastTxAsyncRequest {
            type Response = RpcBroadcastTxAsyncResponse;
            type Error = RpcBroadcastTxAsyncError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!([serialize_signed_transaction(&self.signed_transaction)?]))
            }
        });
    }
}

impl_method! {
    pub mod broadcast_tx_commit {
        pub use near_jsonrpc_primitives::types::transactions::RpcTransactionError;
        pub use near_primitives::transaction::SignedTransaction;

        pub type RpcBroadcastTxCommitResponse = near_primitives::views::FinalExecutionOutcomeView;

        #[derive(Debug)]
        pub struct RpcBroadcastTxCommitRequest {
            pub signed_transaction: SignedTransaction,
        }

        impl From<RpcBroadcastTxCommitRequest>
            for near_jsonrpc_primitives::types::transactions::RpcBroadcastTransactionRequest
        {
            fn from(this: RpcBroadcastTxCommitRequest) -> Self {
                Self {
                    signed_transaction: this.signed_transaction,
                }
            }
        }

        impl_!(RpcMethod for RpcBroadcastTxCommitRequest {
            type Response = RpcBroadcastTxCommitResponse;
            type Error = RpcTransactionError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!([serialize_signed_transaction(&self.signed_transaction)?]))
            }
        });
    }
}

impl_method! {
    pub mod chunk {
        pub use near_jsonrpc_primitives::types::chunks::{RpcChunkError, RpcChunkRequest};

        pub type RpcChunkResponse = near_primitives::views::ChunkView;

        impl RpcHandlerResponse for RpcChunkResponse {}

        impl RpcHandlerError for RpcChunkError {
            fn parse_raw_error(value: serde_json::Value) -> Option<Result<Self, serde_json::Error>> {
                parse_unknown_block!(value => Self)
            }
        }

        impl_!(RpcMethod for RpcChunkRequest {
            type Response = RpcChunkResponse;
            type Error = RpcChunkError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(self))
            }
        });
    }
}

impl_method! {
    pub mod gas_price {
        pub use near_jsonrpc_primitives::types::gas_price::{
            RpcGasPriceError, RpcGasPriceRequest,
        };

        pub type RpcGasPriceResponse = near_primitives::views::GasPriceView;

        impl RpcHandlerResponse for RpcGasPriceResponse {}

        impl RpcHandlerError for RpcGasPriceError {
            fn parse_raw_error(value: serde_json::Value) -> Option<Result<Self, serde_json::Error>> {
                parse_unknown_block!(value => Self)
            }
        }

        impl_!(RpcMethod for RpcGasPriceRequest {
            type Response = RpcGasPriceResponse;
            type Error = RpcGasPriceError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!([self]))
            }
        });
    }
}

impl_method! {
    pub mod health {
        pub use near_jsonrpc_primitives::types::status::{
            RpcHealthResponse, RpcStatusError,
        };

        #[derive(Debug)]
        pub struct RpcHealthRequest;

        impl RpcHandlerResponse for RpcHealthResponse {}

        impl_!(RpcMethod for RpcHealthRequest {
            type Response = RpcHealthResponse;
            type Error = RpcStatusError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(null))
            }
        });
    }
}

impl_method! {
    pub mod light_client_proof {
        pub use near_jsonrpc_primitives::types::light_client::{
            RpcLightClientExecutionProofRequest, RpcLightClientExecutionProofResponse,
            RpcLightClientProofError,
        };

        impl RpcHandlerResponse for RpcLightClientExecutionProofResponse {}

        impl RpcHandlerError for RpcLightClientProofError {
            fn parse_raw_error(value: serde_json::Value) -> Option<Result<Self, serde_json::Error>> {
                parse_unknown_block!(value => Self)
            }
        }

        impl_!(RpcMethod for RpcLightClientExecutionProofRequest {
            type Response = RpcLightClientExecutionProofResponse;
            type Error = RpcLightClientProofError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(self))
            }
        });
    }
}

impl_method! {
    pub mod next_light_client_block {
        pub use near_jsonrpc_primitives::types::light_client::{
            RpcLightClientNextBlockError, RpcLightClientNextBlockRequest,
        };
        pub use near_primitives::views::LightClientBlockView;
        pub type RpcLightClientNextBlockResponse = Option<LightClientBlockView>;

        impl RpcHandlerResponse for RpcLightClientNextBlockResponse {}

        impl RpcHandlerError for RpcLightClientNextBlockError {
            fn parse_raw_error(value: serde_json::Value) -> Option<Result<Self, serde_json::Error>> {
                parse_unknown_block!(value => Self)
            }
        }

        impl_!(RpcMethod for RpcLightClientNextBlockRequest {
            type Response = RpcLightClientNextBlockResponse;
            type Error = RpcLightClientNextBlockError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(self))
            }
        });
    }
}

impl_method! {
    pub mod network_info {
        pub use near_jsonrpc_primitives::types::network_info::{RpcNetworkInfoError, RpcNetworkInfoResponse};

        #[derive(Debug)]
        pub struct RpcNetworkInfoRequest;

        impl RpcHandlerResponse for RpcNetworkInfoResponse {}

        impl RpcHandlerError for RpcNetworkInfoError {}

        impl_!(RpcMethod for RpcNetworkInfoRequest {
            type Response = RpcNetworkInfoResponse;
            type Error = RpcNetworkInfoError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(null))
            }
        });
    }
}

impl_method! {
    pub mod query {
        pub use near_jsonrpc_primitives::types::query::{
            RpcQueryError, RpcQueryRequest, RpcQueryResponse,
        };

        impl RpcHandlerResponse for RpcQueryResponse {}

        impl RpcHandlerError for RpcQueryError {}

        impl_!(RpcMethod for RpcQueryRequest {
            type Response = RpcQueryResponse;
            type Error = RpcQueryError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(self))
            }
        });
    }
}

impl_method! {
    pub mod status {
        pub use near_jsonrpc_primitives::types::status::RpcStatusError;

        pub type RpcStatusResponse = near_primitives::views::StatusResponse;

        #[derive(Debug)]
        pub struct RpcStatusRequest;

        impl RpcHandlerResponse for RpcStatusResponse {}

        impl_!(RpcMethod for RpcStatusRequest {
            type Response = RpcStatusResponse;
            type Error = RpcStatusError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(null))
            }
        });
    }
}

impl_method! {
    pub mod tx {
        pub use near_jsonrpc_primitives::types::transactions::RpcTransactionError;
        pub use near_jsonrpc_primitives::types::transactions::TransactionInfo;

        pub type RpcTransactionStatusResponse = near_primitives::views::FinalExecutionOutcomeView;

        #[derive(Debug)]
        pub struct RpcTransactionStatusRequest {
            pub transaction_info: TransactionInfo,
        }

        impl From<RpcTransactionStatusRequest>
            for near_jsonrpc_primitives::types::transactions::RpcTransactionStatusCommonRequest
        {
            fn from(this: RpcTransactionStatusRequest) -> Self {
                Self {
                    transaction_info: this.transaction_info,
                }
            }
        }

        impl_!(RpcMethod for RpcTransactionStatusRequest {
            type Response = RpcTransactionStatusResponse;
            type Error = RpcTransactionError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(
                    match &self.transaction_info {
                        TransactionInfo::Transaction(signed_transaction) => {
                            json!([serialize_signed_transaction(signed_transaction)?])
                        }
                        TransactionInfo::TransactionId { hash, account_id } => {
                            json!([hash, account_id])
                        }
                    }
                )
            }
        });
    }
}

impl_method! {
    pub mod validators {
        pub use near_jsonrpc_primitives::types::validator::{
            RpcValidatorError, RpcValidatorRequest,
        };

        pub type RpcValidatorResponse = near_primitives::views::EpochValidatorInfo;

        impl RpcHandlerResponse for RpcValidatorResponse {}

        impl_!(RpcMethod for RpcValidatorRequest {
            type Response = RpcValidatorResponse;
            type Error = RpcValidatorError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(self))
            }
        });
    }
}

impl_method! {
    pub mod EXPERIMENTAL_broadcast_tx_sync {
        pub use near_jsonrpc_primitives::types::transactions::{
            RpcBroadcastTxSyncResponse, RpcTransactionError,
        };
        pub use near_primitives::transaction::SignedTransaction;

        #[derive(Debug)]
        pub struct RpcBroadcastTxSyncRequest {
            pub signed_transaction: SignedTransaction,
        }

        impl From<RpcBroadcastTxSyncRequest>
            for near_jsonrpc_primitives::types::transactions::RpcBroadcastTransactionRequest
        {
            fn from(this: RpcBroadcastTxSyncRequest) -> Self {
                Self { signed_transaction: this.signed_transaction }
            }
        }

        impl_!(RpcMethod for RpcBroadcastTxSyncRequest {
            type Response = RpcBroadcastTxSyncResponse;
            type Error = RpcTransactionError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!([serialize_signed_transaction(&self.signed_transaction)?]))
            }
        });
    }
}

impl_method! {
    pub mod EXPERIMENTAL_changes {
        pub use near_jsonrpc_primitives::types::changes::{
            RpcStateChangesError, RpcStateChangesInBlockByTypeRequest,
            RpcStateChangesInBlockResponse,
        };

        impl RpcHandlerResponse for RpcStateChangesInBlockResponse {}

        impl_!(RpcMethod for RpcStateChangesInBlockByTypeRequest {
            type Response = RpcStateChangesInBlockResponse;
            type Error = RpcStateChangesError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(self))
            }
        });
    }
}

impl_method! {
    pub mod EXPERIMENTAL_changes_in_block {
        pub use near_jsonrpc_primitives::types::changes::{
            RpcStateChangesError, RpcStateChangesInBlockRequest,
            RpcStateChangesInBlockByTypeResponse,
        };

        impl RpcHandlerResponse for RpcStateChangesInBlockByTypeResponse {}

        impl_!(RpcMethod for RpcStateChangesInBlockRequest {
            type Response = RpcStateChangesInBlockByTypeResponse;
            type Error = RpcStateChangesError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(self))
            }
        });
    }
}

impl_method! {
    pub mod EXPERIMENTAL_check_tx {
        pub use near_jsonrpc_primitives::types::transactions::{
            RpcBroadcastTxSyncResponse, RpcTransactionError,
        };
        pub use near_primitives::transaction::SignedTransaction;

        #[derive(Debug)]
        pub struct RpcCheckTxRequest {
            pub signed_transaction: SignedTransaction,
        }

        impl From<RpcCheckTxRequest>
            for near_jsonrpc_primitives::types::transactions::RpcBroadcastTransactionRequest
        {
            fn from(this: RpcCheckTxRequest) -> Self {
                Self { signed_transaction: this.signed_transaction }
            }
        }

        impl_!(RpcMethod for RpcCheckTxRequest {
            type Response = RpcBroadcastTxSyncResponse;
            type Error = RpcTransactionError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!([serialize_signed_transaction(&self.signed_transaction)?]))
            }
        });
    }
}

impl_method! {
    pub mod EXPERIMENTAL_genesis_config {
        pub type RpcGenesisConfigResponse = near_chain_configs::GenesisConfig;

        #[derive(Debug)]
        pub struct RpcGenesisConfigRequest;

        #[derive(Debug, Deserialize, Error)]
        #[error("{}", unreachable!("fatal: this error should never be constructed"))]
        pub enum RpcGenesisConfigError {}

        impl RpcHandlerResponse for RpcGenesisConfigResponse {}

        impl RpcHandlerError for RpcGenesisConfigError {}

        impl_!(RpcMethod for RpcGenesisConfigRequest {
            type Response = RpcGenesisConfigResponse;
            type Error = RpcGenesisConfigError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(null))
            }
        });
    }
}

impl_method! {
    pub mod EXPERIMENTAL_protocol_config {
        pub use near_jsonrpc_primitives::types::config::{
            RpcProtocolConfigError, RpcProtocolConfigRequest,
        };

        pub type RpcProtocolConfigResponse = near_chain_configs::ProtocolConfigView;

        impl RpcHandlerResponse for RpcProtocolConfigResponse {}

        impl RpcHandlerError for RpcProtocolConfigError {
            fn parse_raw_error(value: serde_json::Value) -> Option<Result<Self, serde_json::Error>> {
                parse_unknown_block!(value => Self)
            }
        }

        impl_!(RpcMethod for RpcProtocolConfigRequest {
            type Response = RpcProtocolConfigResponse;
            type Error = RpcProtocolConfigError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(self))
            }
        });
    }
}

impl_method! {
    pub mod EXPERIMENTAL_receipt {
        pub use near_jsonrpc_primitives::types::receipts::{
            RpcReceiptError, RpcReceiptRequest,
        };

        pub type RpcReceiptResponse = near_primitives::views::ReceiptView;

        impl RpcHandlerResponse for RpcReceiptResponse {}

        impl RpcHandlerError for RpcReceiptError {}

        impl_!(RpcMethod for RpcReceiptRequest {
            type Response = RpcReceiptResponse;
            type Error = RpcReceiptError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(self))
            }
        });
    }
}

impl_method! {
    pub mod EXPERIMENTAL_tx_status {
        pub use near_jsonrpc_primitives::types::transactions::RpcTransactionError;
        pub use near_jsonrpc_primitives::types::transactions::TransactionInfo;

        pub type RpcTransactionStatusResponse = near_primitives::views::FinalExecutionOutcomeWithReceiptView;

        #[derive(Debug)]
        pub struct RpcTransactionStatusRequest {
            pub transaction_info: TransactionInfo,
        }

        impl From<RpcTransactionStatusRequest>
            for near_jsonrpc_primitives::types::transactions::RpcTransactionStatusCommonRequest
        {
            fn from(this: RpcTransactionStatusRequest) -> Self {
                Self {
                    transaction_info: this.transaction_info,
                }
            }
        }

        impl RpcHandlerResponse for RpcTransactionStatusResponse {}

        impl_!(RpcMethod for RpcTransactionStatusRequest {
            type Response = RpcTransactionStatusResponse;
            type Error = RpcTransactionError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(
                    match &self.transaction_info {
                        TransactionInfo::Transaction(signed_transaction) => {
                            json!([serialize_signed_transaction(signed_transaction)?])
                        }
                        TransactionInfo::TransactionId { hash, account_id } => {
                            json!([hash, account_id])
                        }
                    }
                )
            }
        });
    }
}

impl_method! {
    pub mod EXPERIMENTAL_validators_ordered {
        pub use near_jsonrpc_primitives::types::validator::{
            RpcValidatorError, RpcValidatorsOrderedRequest, RpcValidatorsOrderedResponse,
        };

        impl RpcHandlerResponse for RpcValidatorsOrderedResponse {}

        impl_!(RpcMethod for RpcValidatorsOrderedRequest {
            type Response = RpcValidatorsOrderedResponse;
            type Error = RpcValidatorError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(self))
            }
        });
    }
}

#[cfg(feature = "sandbox")]
impl_method! {
    pub mod sandbox_patch_state {
        pub use near_jsonrpc_primitives::types::sandbox::{
            RpcSandboxPatchStateError, RpcSandboxPatchStateRequest,
            RpcSandboxPatchStateResponse,
        };

        impl RpcHandlerResponse for RpcSandboxPatchStateResponse {}

        impl RpcHandlerError for RpcSandboxPatchStateError {}

        impl_!(RpcMethod for RpcSandboxPatchStateRequest {
            type Response = RpcSandboxPatchStateResponse;
            type Error = RpcSandboxPatchStateError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(self))
            }
        });
    }
}

#[cfg(feature = "sandbox")]
impl_method! {
    pub mod sandbox_fast_forward {
        pub use near_jsonrpc_primitives::types::sandbox::{
            RpcSandboxFastForwardError, RpcSandboxFastForwardRequest,
            RpcSandboxFastForwardResponse,
        };

        impl RpcHandlerResponse for RpcSandboxFastForwardResponse {}

        impl RpcHandlerError for RpcSandboxFastForwardError {}

        impl_!(RpcMethod for RpcSandboxFastForwardRequest {
            type Response = RpcSandboxFastForwardResponse;
            type Error = RpcSandboxFastForwardError;

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(self))
            }
        });
    }
}

#[cfg(feature = "adversarial")]
impl_method! {
    pub mod adv_set_weight {
        #[derive(Debug)]
        pub struct RpcAdversarialSetWeightRequest { pub height: u64 }

        impl_!(RpcMethod for RpcAdversarialSetWeightRequest {
            type Response = ();
            type Error = ();

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(self.height))
            }
        });
    }
}

#[cfg(feature = "adversarial")]
impl_method! {
    pub mod adv_disable_header_sync {
        #[derive(Debug)]
        pub struct RpcAdversarialDisableHeaderSyncRequest;

        impl_!(RpcMethod for RpcAdversarialDisableHeaderSyncRequest {
            type Response = ();
            type Error = ();

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(null))
            }
        });
    }
}

#[cfg(feature = "adversarial")]
impl_method! {
    pub mod adv_disable_doomslug {
        #[derive(Debug)]
        pub struct RpcAdversarialDisableDoomslugRequest;

        impl_!(RpcMethod for RpcAdversarialDisableDoomslugRequest {
            type Response = ();
            type Error = ();

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(null))
            }
        });
    }
}

#[cfg(feature = "adversarial")]
impl_method! {
    pub mod adv_produce_blocks {
        #[derive(Debug)]
        pub struct RpcAdversarialProduceBlocksRequest {
            pub num_blocks: u64,
            pub only_valid: bool,
        }

        impl_!(RpcMethod for RpcAdversarialProduceBlocksRequest {
            type Response = ();
            type Error = ();

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!([self.num_blocks, self.only_valid]))
            }
        });
    }
}

#[cfg(feature = "adversarial")]
impl_method! {
    pub mod adv_switch_to_height {
        #[derive(Debug)]
        pub struct RpcAdversarialSwitchToHeightRequest { pub height: u64 }

        impl_!(RpcMethod for RpcAdversarialSwitchToHeightRequest {
            type Response = ();
            type Error = ();

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!([self.height]))
            }
        });
    }
}

#[cfg(feature = "adversarial")]
impl_method! {
    pub mod adv_get_saved_blocks {
        use serde::Deserialize;

        #[derive(Debug)]
        pub struct RpcAdversarialGetSavedBlocksRequest;

        #[derive(Debug, Deserialize)]
        pub struct RpcAdversarialGetSavedBlocksResponse(pub u64);

        impl RpcHandlerResponse for RpcAdversarialGetSavedBlocksResponse {}

        impl_!(RpcMethod for RpcAdversarialGetSavedBlocksRequest {
            type Response = RpcAdversarialGetSavedBlocksResponse;
            type Error = ();

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(null))
            }
        });
    }
}

#[cfg(feature = "adversarial")]
impl_method! {
    pub mod adv_check_store {
        use serde::Deserialize;

        #[derive(Debug)]
        pub struct RpcAdversarialCheckStoreRequest;

        #[derive(Debug, Deserialize)]
        pub struct RpcAdversarialCheckStoreResponse(pub u64);

        impl RpcHandlerResponse for RpcAdversarialCheckStoreResponse {}

        impl_!(RpcMethod for RpcAdversarialCheckStoreRequest {
            type Response = RpcAdversarialCheckStoreResponse;
            type Error = ();

            fn params(&self) -> Result<serde_json::Value, io::Error> {
                Ok(json!(null))
            }
        });
    }
}
