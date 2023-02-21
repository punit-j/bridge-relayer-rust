//! Operations with eth contact
//!
//! # Example
//!
//! ```
//! let abi = fs::read("/home/misha/trash/abi.json").unwrap();
//! let priv_key = secp256k1::SecretKey::from_str(&(fs::read_to_string("/home/misha/trash/acc2prk").unwrap().as_str())[..64]).unwrap();
//! let contract_addr = web3::types::Address::from_str("5c739e4039D552E2DBF94ce9E7Db261c88BcEc84").unwrap();
//! let token_addr = web3::types::Address::from_str("b2d75C5a142A68BDA438e6a318C7FBB2242f9693").unwrap();
//!
//! let eth = RainbowBridgeEthereumClient::new("https://goerli.infura.io/v3/05155f003f604cd884bfd577c2219da5",
//!                                            "/home/misha/trash/rr/rainbow-bridge/cli/index.js",
//!                                            contract_addr,
//!                                            &*abi, priv_key).unwrap();
//!
//! let res = eth.transfer_token(token_addr,
//!                              web3::types::Address::from_str("2a23E0Fa3Afe77AFf5dc6c6a007E3A10c1450633").unwrap(),
//!                              159, web3::types::U256::from(200)).await;
//! println!("transfer_token hash {:?}", &res);
//! let tx_hash = res.unwrap();
//!
//! // wait for transaction process
//! let res = loop {
//!     sleep(time::Duration::from_secs(2));
//!     let res = eth.transaction_status(tx_hash.clone()).await.unwrap();
//!     if res == transactions::TransactionStatus::Pengind {
//!         continue;
//!     }
//!
//!     break res;
//! };
//!
//! // get proof
//! if res == transactions::TransactionStatus::Sucess {
//!     let proof = eth.get_proof(&tx_hash).await;
//!     println!("proof {:?}", proof);
//! }
//! else {
//!     println!("Transaction is failure");
//! }
//! ```

pub mod proof;
pub mod transactions;

use eth_client::methods::new_eth_rpc_client;
use web3::{api::Namespace, transports::Http};

pub struct RainbowBridgeEthereumClient<'a> {
    api_url: http_types::Url,
    rainbow_bridge_index: &'a str,
    client: web3::api::Eth<Http>,
}

impl<'a> RainbowBridgeEthereumClient<'a> {
    pub fn new(
        eth_endpoint: http_types::Url,
        rainbow_bridge_index: &'a str,
        rpc_timeout_secs: u64,
    ) -> Result<Self, std::string::String> {
        let transport = web3::transports::Http::with_client(
            new_eth_rpc_client(Some(std::time::Duration::from_secs(rpc_timeout_secs)))
                .map_err(|e| e.to_string())?,
            eth_endpoint.clone(),
        );
        let client = web3::api::Eth::new(transport);

        Ok(Self {
            api_url: eth_endpoint,
            rainbow_bridge_index,
            client,
        })
    }

    pub async fn transaction_status(
        &self,
        tx_hash: web3::types::H256,
    ) -> web3::error::Result<transactions::TransactionStatus> {
        transactions::transaction_status(&self.client, tx_hash).await
    }

    pub async fn get_proof<'b, 'c>(
        &self,
        tx_hash: &'b web3::types::H256,
    ) -> Result<fast_bridge_common::Proof, proof::Error<'c>> {
        proof::get_proof(
            self.api_url.as_str(),
            &self.client,
            self.rainbow_bridge_index,
            tx_hash,
        )
        .await
    }
}

#[cfg(test)]
pub mod tests {
    use crate::config::default_rpc_timeout_secs;
    use crate::ethereum::transactions::TransactionStatus;
    use crate::ethereum::RainbowBridgeEthereumClient;
    use crate::test_utils::get_rb_index_path_str;
    use eth_client::test_utils::get_eth_rpc_url;
    use url::Url;
    use web3::types::U64;

    async fn get_params() -> (Url, String) {
        (get_eth_rpc_url(), get_rb_index_path_str())
    }

    #[tokio::test]
    async fn smoke_new_test() {
        let (eth1_endpoint, rb_index_path_str) = get_params().await;
        let _eth = RainbowBridgeEthereumClient::new(
            eth1_endpoint,
            &rb_index_path_str,
            default_rpc_timeout_secs(),
        )
        .unwrap();
    }

    #[tokio::test]
    async fn smoke_get_proof_test() {
        let (eth1_endpoint, rb_index_path_str) = get_params().await;
        let eth = RainbowBridgeEthereumClient::new(
            eth1_endpoint,
            &rb_index_path_str,
            default_rpc_timeout_secs(),
        )
        .unwrap();

        let tx_hash = web3::types::H256::from_slice(
            &hex::decode("cb50c668e750650fc53d0027112d0580b42f3b658780598cb6899344e2b94183")
                .unwrap(),
        );

        let proof = eth.get_proof(&tx_hash).await.unwrap();
        println!("{:?}", proof);
    }

    #[tokio::test]
    async fn smoke_transaction_status_test() {
        let (eth1_endpoint, rb_index_path_str) = get_params().await;
        let eth = RainbowBridgeEthereumClient::new(
            eth1_endpoint,
            &rb_index_path_str,
            default_rpc_timeout_secs(),
        )
        .unwrap();

        let tx_hash = web3::types::H256::from_slice(
            &hex::decode("564e7a804e74e45710021c692a0fdc2ef5284bc4fbfd3b552b359adb89e21f14")
                .unwrap(),
        );
        let tx_status = eth.transaction_status(tx_hash).await.unwrap();

        assert_eq!(tx_status, TransactionStatus::Success(U64::from(8180335)));
    }
}
