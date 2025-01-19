use std::str::FromStr;

use crate::constant::constants::{DECIMALS, EKUBO_TICK_SIZE, LIQUIDITY_LOCK_FOREVER_TIMESTAMP, QUOTE_TOKENS};
use crate::utils::types::fraction::Rounding;

use super::call::{parse_u256_from_felts, AggregateError};
use super::types::ekubo::{Bound, Bounds, EkuboLiquidityLockPosition, EkuboMemecoin, Liquidity, PoolKey};
use super::types::fraction::Fraction;
use num_bigint::BigUint;
use num_traits::{FromPrimitive, One};
use starknet::core::types::{BlockId, BlockTag, FunctionCall};
use starknet::macros::selector;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, Provider};
use starknet_core::types::Felt;
use url::Url;

fn get_provider() -> Result<JsonRpcClient<HttpTransport>, AggregateError> {
    Ok(JsonRpcClient::new(HttpTransport::new(
        Url::parse("https://starknet-mainnet.public.blastapi.io/rpc/v0_7")
            .map_err(AggregateError::Url)?
    )))
}

#[derive(Debug, Clone)]
pub struct LiquidityParams {
    pub is_quote_token_safe: bool,
    pub parsed_starting_mcap: String,
}

pub async fn get_ekubo_liquidity_lock_position(
    liquidity: &Liquidity
) -> Result<(EkuboLiquidityLockPosition), Box<dyn std::error::Error>> {
    let provider = get_provider()?;
    // Call the contract to get the details
    let call_result = match provider
    .call(
        FunctionCall {
            contract_address: Felt::from_hex(&liquidity.launch_manager)
                .map_err(|e| AggregateError::ContractCall(format!("Invalid address: {}", e)))?,
            entry_point_selector: selector!("liquidity_position_details"),
            calldata: vec![Felt::from_hex(&liquidity.ekubo_id)?],
        },
        BlockId::Tag(BlockTag::Latest),
    )
    .await {
        Ok(result) => {
            println!("Contract call successful!");
            result
        }
        Err(e) => {
            println!("Contract call failed: {:?}", e);
            return Err(Box::new(AggregateError::ContractCall(format!("Contract call failed: {:?}", e))));
        }
    };

    Ok(EkuboLiquidityLockPosition {
        unlock_time: LIQUIDITY_LOCK_FOREVER_TIMESTAMP,
        owner: call_result[0].to_hex_string(),
        pool_key: PoolKey {
            token0: call_result[2].to_hex_string(),
            token1: call_result[3].to_hex_string(),
            fee: call_result[4].to_hex_string(),
            tick_spacing: call_result[5].to_hex_string(),
            extension: call_result[6].to_hex_string(),
        },
        bounds: Bounds {
            lower: Bound {
                mag: call_result[7].to_string(),
                sign: call_result[8].to_string(),
            },
            upper: Bound {
                mag: call_result[9].to_string(),
                sign: call_result[10].to_string(),
            },
        },
    })
}
pub async fn get_price(pair: String, block_identifier: BlockId) -> Result<Fraction, Box<dyn std::error::Error>> {
    if pair == "" {return Ok(Fraction::new(BigUint::from(10u64).pow(DECIMALS), Some(BigUint::one()))?)}

    let provider = get_provider()?;
    let call_result = match provider
        .call(
            FunctionCall {
                contract_address: Felt::from_hex(&pair)
                    .map_err(|e| AggregateError::ContractCall(format!("Invalid address: {}", e)))?,
                entry_point_selector: selector!("get_reserves"),
                calldata: vec![],
            },
            block_identifier,
        )
        .await {
            Ok(result) => {
                result
            }
            Err(e) => {
                println!("Contract call failed: {:?}", e);
                return Err(Box::new(AggregateError::ContractCall(format!("Contract call failed: {:?}", e))));
            }
        };
        let reserve0 = if let (Some(low), Some(high)) = (call_result.get(0), call_result.get(1)) {
            BigUint::from_str(&parse_u256_from_felts(low, high))?
        } else {
            eprintln!("Failed to decode reserve0");
            return Err(Box::new(AggregateError::ContractCall("Failed to decode reserve0".to_string())));
        };
        let reserve1 = if let (Some(low), Some(high)) = (call_result.get(2), call_result.get(3)) {
            BigUint::from_str(&parse_u256_from_felts(low, high))?
        } else {
            eprintln!("Failed to decode reserve1");
            return Err(Box::new(AggregateError::ContractCall("Failed to decode reserve1".to_string())));
        };

        // println!("{}", reserve0);
        // println!("{}", reserve1);

    // Perform the fraction operation (reserve1 / reserve0) * 10^12 for scaling
    let scale = BigUint::from(10u64).pow(12);
    let fraction = Fraction::new(reserve1, Some(reserve0))? * Fraction::new(scale.clone(), Some(BigUint::one()))?;
    Ok(fraction)
}

pub fn get_initial_price(starting_tick: i64) -> f64 {
    let log_tick_size = EKUBO_TICK_SIZE.ln();
    (starting_tick as f64) * log_tick_size   
}

pub async fn parse_liquidity_params(memecoin: &EkuboMemecoin) -> Result<LiquidityParams, Box<dyn std::error::Error>> {
    // println!("{:?}", memecoin);
    
    // Quote token info check
    let quote_token_infos = QUOTE_TOKENS.get(&memecoin.liquidity.quote_token as &str);
    let is_quote_token_safe = quote_token_infos.is_some();

    // Get Ether price at launch
    let quote_token_price_at_launch = get_price(quote_token_infos.unwrap().usdc_pair.to_string(),starknet::core::types::BlockId::Number(memecoin.launch.block_number)).await?;
    // println!("{:?}", quote_token_price_at_launch);
    
    // Calculate initial price and starting market cap
    let initial_price = get_initial_price(memecoin.liquidity.starting_tick);
    // println!("{:?}", initial_price);
    
    // Now we can safely convert the scaled price to BigUint
    let price = BigUint::from_f64(initial_price).unwrap();

    // println!("{:?}", price);

    let starting_mcap = if is_quote_token_safe {

        let supply = Fraction::new(memecoin.total_supply.clone(), Some(BigUint::from(1u64)))?;
        // println!("{:?}", supply);
        let decimals = Fraction::new(BigUint::from(10u64.pow(DECIMALS as u32)), Some(BigUint::from(1u64)))?* Fraction::new(BigUint::from(10u64).pow(48), Some(BigUint::one()))?;
        // println!("{:?}", decimals);
        let price_fraction = Fraction::new(price, Some(BigUint::one()))?* Fraction::new(BigUint::from(10u64).pow(DECIMALS), Some(BigUint::one()))?;
        // println!("{:?}", price_fraction);
        Some((price_fraction * quote_token_price_at_launch *supply)
                           /decimals)
    } else {
        None
    };

    let starting_mcap_value = starting_mcap.unwrap()?;

    // println!("{:?}", starting_mcap_value.to_formatted_string());

    // Format the starting market cap
    let parsed_starting_mcap = starting_mcap_value.to_significant_digits(0, Rounding::RoundDown)?;

    // println!("{}", parsed_starting_mcap);

    Ok(LiquidityParams {
        is_quote_token_safe,
        parsed_starting_mcap,
    })
}