use std::sync::{Arc, Mutex};

pub async fn execute_transfer(
    key: impl web3::signing::Key,
    transfer_message: spectre_bridge_common::Event,
    contract_abi: &[u8],
    rpc_url: &str,
    contract_addr: web3::types::Address,
    profit_threshold: f64,
    settings: Arc<Mutex<crate::Settings>>,
) -> Result<Option<web3::types::H256>, String> {
    let method_name = "transferTokens";
    let transfer_message = if let spectre_bridge_common::Event::SpectreBridgeTransferEvent {
        nonce,
        chain_id,
        valid_till,
        transfer,
        fee,
        recipient,
    } = transfer_message
    {
        (nonce, chain_id, valid_till, transfer, fee, recipient)
    } else {
        return Err("Received invalid Event".into());
    };

    let token = web3::types::Address::from(transfer_message.3.token_eth);
    let recipient = web3::types::Address::from(transfer_message.5);
    let nonce = web3::types::U256::from(transfer_message.0 .0);
    let amount = web3::types::U256::from(transfer_message.3.amount.0);
    let method_args = (token, recipient, nonce, amount);

    let estimated_gas_in_wei = eth_client::methods::estimate_gas(
        rpc_url,
        key.address(),
        contract_abi,
        method_name,
        method_args,
    )
    .await;
    match estimated_gas_in_wei {
        Ok(_) => (),
        Err(error) => return Err(format!("Failed to estimate gas in WEI: {}", error)),
    }

    let gas_price_in_wei = eth_client::methods::gas_price(rpc_url).await;
    match gas_price_in_wei {
        Ok(_) => (),
        Err(error) => return Err(format!("Failed to fetch gas price in WEI: {}", error)),
    }

    let eth_price_in_usd = eth_client::methods::eth_price().await;
    match eth_price_in_usd {
        Ok(price) => match price {
            Some(_) => (),
            None => {
                return Err("Failed to fetch Ethereum price in USD: Invalid coin id".to_string())
            }
        },
        Err(error) => return Err(format!("Failed to fetch Ethereum price in USD: {}", error)),
    }

    let estimated_transfer_execution_price = eth_client::methods::estimate_transfer_execution(
        estimated_gas_in_wei.unwrap(),
        gas_price_in_wei.unwrap(),
        eth_price_in_usd.unwrap().unwrap(),
    );

    let fee_token = transfer_message.4.token;
    let fee_amount = web3::types::U256::from(transfer_message.4.amount.0);

    let coin_id = settings
        .lock()
        .unwrap()
        .near_tokens_coin_id
        .get_coin_id(fee_token);
    match coin_id {
        Some(_) => (),
        None => {
            return Err(format!(
                "Failed to get coin id ({}) by matching",
                coin_id.unwrap()
            ))
        }
    }

    let fee_token_usd = eth_client::methods::token_price(coin_id.unwrap()).await;
    match fee_token_usd {
        Ok(price) => match price {
            Some(_) => (),
            None => return Err("Failed to get token price: Invalid coin id".to_string()),
        },
        Err(error) => return Err(format!("Failed to get token price: {}", error)),
    }

    let profit = crate::profit_estimation::get_profit(
        fee_token_usd.unwrap().unwrap(),
        fee_amount,
        estimated_transfer_execution_price,
    )
    .await;

    println!("Profit for nonce {:?} is {}, threshold: {}", nonce, profit, profit_threshold);

    match profit > profit_threshold {
        true => {
            let tx_hash = eth_client::methods::change(
                rpc_url,
                contract_addr,
                contract_abi,
                method_name,
                method_args,
                key,
            )
            .await;
            match tx_hash {
                Ok(hash) => Ok(Some(hash)),
                Err(error) => Err(format!("Failed to execute tokens transfer: {}", error)),
            }
        }
        false => Ok(None),
    }
}
