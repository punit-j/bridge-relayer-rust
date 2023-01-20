use crate::config::{NearTokenInfo, Settings};
use crate::logs::EVENT_PROCESSOR_TARGET;
use near_sdk::AccountId;
use fast_bridge_common::TransferMessage;
use std::sync::{Arc, Mutex};
use web3::types::{H160, U256};

pub async fn execute_transfer(
    relay_key_on_eth: impl web3::signing::Key,
    transfer_event: fast_bridge_common::Event,
    eth_erc20_fast_bridge_contract_abi: &[u8],
    eth1_rpc_url: &str,
    eth_erc20_fast_bridge_proxy_contract_addr: web3::types::Address,
    profit_threshold: Option<f64>,
    settings: Arc<Mutex<Settings>>,
    near_relay_account_id: String,
    transaction_count: web3::types::U256,
) -> Result<web3::types::H256, crate::errors::CustomError> {
    let (nonce, method_name, method_args, transfer_message) =
        get_transfer_data(transfer_event, near_relay_account_id)?;

    let estimated_gas = eth_client::methods::estimate_gas(
        eth1_rpc_url,
        relay_key_on_eth.address(),
        eth_erc20_fast_bridge_proxy_contract_addr,
        eth_erc20_fast_bridge_contract_abi,
        method_name.as_str(),
        method_args.clone(),
    )
    .await;

    let estimated_gas = match estimated_gas {
        Ok(gas) => gas,
        Err(error) => return Err(crate::errors::CustomError::FailedEstimateGas(error)),
    };

    if transfer_message.fee.token != transfer_message.transfer.token_near {
        return Err(crate::errors::CustomError::InvalidFeeToken);
    }

    let token_info = get_near_token_info(settings.clone(), transfer_message.transfer.token_near)?;

    if token_info.eth_address != transfer_message.transfer.token_eth.into() {
        return Err(crate::errors::CustomError::InvalidEthTokenAddress);
    }

    let min_fee_allowed = estimate_min_fee(&token_info, transfer_message.transfer.amount.0)
        .ok_or(crate::errors::CustomError::FailedFeeCalculation)?;

    if transfer_message.fee.amount.0 < min_fee_allowed {
        return Err(crate::errors::CustomError::NotEnoughFeeToken(
            transfer_message.fee.amount.0,
            min_fee_allowed,
        ));
    }

    if let Some(profit_threshold) = profit_threshold {
        let profit = estimate_profit(
            eth1_rpc_url,
            token_info.clone(),
            transfer_message.fee.amount.0.into(),
            estimated_gas,
        )
        .await?
        .ok_or(crate::errors::CustomError::FailedProfitEstimation)?;

        tracing::info!(
            target: EVENT_PROCESSOR_TARGET,
            "Profit for nonce {:?} is {}, threshold: {}",
            nonce,
            profit,
            profit_threshold
        );

        if profit < profit_threshold {
            return Err(crate::errors::CustomError::TxNotProfitable(
                profit,
                profit_threshold,
            ));
        }
    }

    let tx_hash = eth_client::methods::change(
        eth1_rpc_url,
        eth_erc20_fast_bridge_proxy_contract_addr,
        eth_erc20_fast_bridge_contract_abi,
        &method_name,
        method_args,
        relay_key_on_eth,
        true,
        Some(transaction_count),
        Some(estimated_gas),
        settings.lock().unwrap().max_priority_fee_per_gas
    )
    .await;

    match tx_hash {
        Ok(hash) => Ok(hash),
        Err(error) => Err(crate::errors::CustomError::FailedExecuteTransferTokens(
            error,
        )),
    }
}

fn estimate_min_fee(token_info: &NearTokenInfo, token_amount: u128) -> Option<u128> {
    Some(
        rug::Float::with_val(128, token_amount)
            .mul_add(
                &rug::Float::with_val(64, token_info.percent_fee),
                &rug::Float::with_val(64, token_info.fixed_fee.0),
            )
            .to_integer()?
            .to_u128()?,
    )
}

async fn estimate_profit(
    eth1_rpc_url: &str,
    token_info: NearTokenInfo,
    fee_amount: U256,
    estimated_gas: U256,
) -> Result<Option<f64>, crate::errors::CustomError> {
    let gas_price_in_wei = eth_client::methods::gas_price_wei(eth1_rpc_url).await;
    match gas_price_in_wei {
        Ok(_) => (),
        Err(error) => return Err(crate::errors::CustomError::FailedFetchGasPrice(error)),
    }

    let eth_price_in_usd = eth_client::methods::eth_price_usd().await;
    match eth_price_in_usd {
        Ok(price) => match price {
            Some(_) => (),
            None => return Err(crate::errors::CustomError::FailedFetchEthereumPriceInvalidCoinId),
        },
        Err(error) => return Err(crate::errors::CustomError::FailedFetchEthereumPrice(error)),
    }

    let estimated_transfer_execution_price = eth_client::methods::estimate_transfer_execution_usd(
        estimated_gas,
        gas_price_in_wei.unwrap(),
        eth_price_in_usd.unwrap().unwrap(),
    );

    let fee_token_usd = eth_client::methods::token_price_usd(token_info.exchange_id).await;
    match fee_token_usd {
        Ok(price) => match price {
            Some(_) => (),
            None => return Err(crate::errors::CustomError::FailedGetTokenPriceInvalidCoinId),
        },
        Err(error) => return Err(crate::errors::CustomError::FailedGetTokenPrice(error)),
    }

    Ok(crate::profit_estimation::get_profit_usd(
        fee_token_usd.unwrap().unwrap(),
        fee_amount,
        token_info.decimals,
        estimated_transfer_execution_price,
    )
    .await)
}

fn get_transfer_data(
    transfer_event: fast_bridge_common::Event,
    near_relay_account_id: String,
) -> Result<
    (
        U256,
        String,
        (H160, H160, U256, U256, String),
        TransferMessage,
    ),
    crate::errors::CustomError,
> {
    let method_name = "transferTokens";
    if let fast_bridge_common::Event::FastBridgeInitTransferEvent {
        nonce,
        sender_id: _,
        transfer_message,
    } = transfer_event
    {
        let token = web3::types::Address::from(transfer_message.transfer.token_eth);
        let recipient = web3::types::Address::from(transfer_message.recipient);
        let nonce = web3::types::U256::from(nonce.0);
        let amount = web3::types::U256::from(transfer_message.transfer.amount.0);
        let method_args = (token, recipient, nonce, amount, near_relay_account_id);

        Ok((
            nonce,
            method_name.to_string(),
            method_args,
            transfer_message,
        ))
    } else {
        Err(crate::errors::CustomError::ReceivedInvalidEvent)
    }
}

fn get_near_token_info(
    settings: Arc<Mutex<Settings>>,
    fee_token: AccountId,
) -> Result<NearTokenInfo, crate::errors::CustomError> {
    let token_info = settings
        .lock()
        .unwrap()
        .near_tokens_whitelist
        .get_token_info(fee_token.clone());

    match token_info {
        Some(coin_id) => Ok(coin_id),
        None => {
            Err(crate::errors::CustomError::FailedGetNearTokenInfoByMatching(fee_token.to_string()))
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::logs::init_logger;
    use crate::test_utils::get_settings;
    use crate::utils::get_tx_count;
    use crate::transfer::execute_transfer;
    use eth_client::test_utils::{
        get_eth_erc20_fast_bridge_contract_abi, get_eth_erc20_fast_bridge_proxy_contract_address,
        get_eth_rpc_url, get_eth_token, get_recipient, get_relay_eth_key,
    };
    use near_client::test_utils::{get_near_signer, get_near_token};
    use near_sdk::json_types::U128;
    use rand::Rng;
    use fast_bridge_common::{
        EthAddress, TransferDataEthereum, TransferDataNear, TransferMessage,
    };
    use web3::signing::Key;
    use crate::async_redis_wrapper::AsyncRedisWrapper;

    #[tokio::test]
    async fn smoke_execute_transfer_test() {
        init_logger();

        let eth1_rpc_url = get_eth_rpc_url();
        let relay_key_on_eth = std::sync::Arc::new(get_relay_eth_key());
        let eth_erc20_fast_bridge_contract_abi = get_eth_erc20_fast_bridge_contract_abi().await;
        let profit_threshold = 0f64;
        let settings = std::sync::Arc::new(std::sync::Mutex::new(get_settings()));

        let redis = AsyncRedisWrapper::connect(settings.clone()).await;
        let arc_redis = redis.new_safe();

        let current_nonce: u128 = rand::thread_rng().gen_range(0, 1000000000);
        let near_relay_account_id = get_near_signer().account_id.to_string();

        let transfer_message = fast_bridge_common::Event::FastBridgeInitTransferEvent {
            nonce: U128::from(current_nonce),
            sender_id: near_relay_account_id.parse().unwrap(),
            transfer_message: TransferMessage {
                valid_till: 0,
                transfer: TransferDataEthereum {
                    token_near: get_near_token(),
                    token_eth: EthAddress::from(get_eth_token()),
                    amount: U128::from(1),
                },
                fee: TransferDataNear {
                    token: get_near_token(),
                    amount: U128::from(1_000_000_000),
                },
                recipient: EthAddress::from(get_recipient()),
                valid_till_block_height: Some(0),
            },
        };

        execute_transfer(
            relay_key_on_eth.clone().as_ref(),
            transfer_message,
            eth_erc20_fast_bridge_contract_abi.as_bytes(),
            eth1_rpc_url.as_str(),
            get_eth_erc20_fast_bridge_proxy_contract_address(),
            Some(profit_threshold),
            settings,
            near_relay_account_id,
            get_tx_count(arc_redis, eth1_rpc_url.clone(), relay_key_on_eth.address()).await.unwrap(),
        )
        .await
        .unwrap();
    }
}
