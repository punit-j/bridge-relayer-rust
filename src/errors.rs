#[derive(thiserror::Error, Debug)]
pub enum CustomError {
    #[error("Received invalid event")]
    ReceivedInvalidEvent,

    #[error("Failed to estimate gas in WEI: {0:?}")]
    FailedEstimateGas(web3::contract::Error),

    #[error("Failed to fetch gas price in WEI: {0:?}")]
    FailedFetchGasPrice(web3::contract::Error),

    #[error("Failed to fetch Ethereum price in USD: invalid coin id")]
    FailedFetchEthereumPriceInvalidCoinId,

    #[error("Failed to fetch Ethereum price in USD: {0:?}")]
    FailedFetchEthereumPrice(reqwest::Error),

    #[error("Failed to get coin id ({0}) by matching")]
    FailedGetCoinIdByMatching(String),

    #[error("Failed to get token price: invalid coin id")]
    FailedGetTokenPriceInvalidCoinId,

    #[error("Failed to get token price: {0:?}")]
    FailedGetTokenPrice(reqwest::Error),

    #[error("Failed to execute transferTokens contract method: {0:?}")]
    FailedExecuteTransferTokens(web3::contract::Error),

    #[error("Failed to execute lp_unlock contract method: {0}")]
    FailedExecuteUnlockTokens(String),

    #[error("Failed to unstore transaction: {0:?}")]
    FailedUnstoreTransaction(redis::RedisError),

    #[error("Failed to get transaction data by hash from set: {0:?}")]
    FailedGetTxData(redis::RedisError),

    #[error("Failed to get queue of transaction hashes: {0:?}")]
    FailedGetTxHashesQueue(redis::RedisError),

    #[error("Failed to store pending transaction: {0:?}")]
    FailedStorePendingTx(redis::RedisError),

    #[error("Failed to unstore pending transaction: {0:?}")]
    FailedUnstorePendingTx(redis::RedisError),

    #[error("Failed to execute last_block_number contract method: {0}")]
    FailedExecuteLastBlockNumber(String),

    #[error("Failed to fetch transaction status: {0:?}")]
    FailedFetchTxStatus(web3::Error),

    #[error("Failed to fetch proof: {0}")]
    FailedFetchProof(String),

    #[error("transferTokens transaction status [Failure]: {0}")]
    FailedTxStatus(String),
}
