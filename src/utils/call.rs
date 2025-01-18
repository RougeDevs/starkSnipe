use super::types::ekubo::{EkuboPoolParameters, Launch, Liquidity, Memecoin, StartingPrice};
use num_traits::cast::ToPrimitive;
use serde::de::value::Error;
use starknet::core::types::{BlockId, BlockTag, FunctionCall, U256};
use starknet::core::utils::{get_selector_from_name, normalize_address, parse_cairo_short_string};
use starknet::macros::selector;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, Provider, ProviderError};
use starknet_core::types::Felt;
use url::Url;

use crate::constant::constants::{
    selector_to_str, Selector, EXCHANGE_ADDRESS, MEMECOIN_FACTORY_ADDRESS,
    MULTICALL_AGGREGATOR_ADDRESS,
};
use crate::utils::event_parser::{parse_and_validate_short_string, u256_to_decimal_str};

trait FromFieldBytes: Sized {
    fn from_field_bytes(bytes: [u8; 32]) -> Self;
}

impl FromFieldBytes for u128 {
    fn from_field_bytes(bytes: [u8; 32]) -> Self {
        let last_sixteen_bytes: [u8; 16] = bytes[16..32]
            .try_into()
            .expect("Slice with incorrect length");
        u128::from_be_bytes(last_sixteen_bytes)
    }
}

const EKUBO_NFT: &str = "EKUBO_NFT";

#[derive(Debug, thiserror::Error)]
pub enum AggregateError {
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("URL parsing error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Contract call failed: {0}")]
    ContractCall(String),

    #[error("Parse error: {0}")]
    Parse(String),
}

fn get_provider() -> Result<JsonRpcClient<HttpTransport>, AggregateError> {
    println!("In get provider");
    // Create provider with error handling
    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse("https://starknet-mainnet.public.blastapi.io/rpc/v0_7")
            .map_err(AggregateError::Url)?,
    ));
    Ok(provider)
}

pub async fn get_aggregate_call_data(address: &str) -> Result<Memecoin, AggregateError> {
    println!("In aggregate call");
    let calls = generate_calls(address);
    let call_result = multicall_contract(calls).await.unwrap();
    // Parse results with error handling
    let parsed_result = parse_call_result(address, call_result).await.unwrap();
    Ok(parsed_result)
}

fn generate_calls(address: &str) -> Vec<starknet_core::types::Felt> {
    println!("In generate call");
    let mut calls: Vec<Felt> = vec![Felt::from(10)];

    let factory_address = MEMECOIN_FACTORY_ADDRESS;
    let ekubo_id: String = 1.to_string();

    let factory_calls = [
        ("is_memecoin", Selector::IsMemecoin),
        ("exchange", Selector::ExchangeAddress),
        ("locked_liquidity", Selector::LockedLiquidity),
    ];

    for (name, selector) in factory_calls {
        calls.push(Felt::from_hex_unchecked(factory_address));
        calls.push(get_selector_from_name(&selector_to_str(selector)).unwrap());
        calls.push(Felt::ONE);
        calls.push(if name == "exchange" {
            Felt::from_dec_str(&ekubo_id).unwrap()
        } else {
            Felt::from_hex_unchecked(address)
        });
    }

    // Add other calls with detailed logging
    let coin_calls = [
        ("name", Selector::Name),
        ("symbol", Selector::Symbol),
        ("total_supply", Selector::TotalSupply),
        ("owner", Selector::Owner),
        ("launched_block", Selector::LaunchedAtBlockNumber),
        ("team_allocation", Selector::GetTeamAllocation),
        (
            "liquidity_params",
            Selector::LaunchedWithLiquidityParameters,
        ),
    ];

    for (_name, selector) in coin_calls {
        calls.push(Felt::from_hex_unchecked(address));
        calls.push(get_selector_from_name(&selector_to_str(selector)).unwrap());
        calls.push(Felt::ZERO);
    }
    calls
}

async fn parse_call_result(address: &str, call_result: Vec<Felt>) -> Result<Memecoin, Error> {
    println!("In parse call");
    let is_memecoin = call_result[3] != Felt::ZERO;
    let exchange = normalize_address(Felt::from_bytes_be(&call_result[5].to_bytes_be()))
        .to_hex_string()
        .eq(EXCHANGE_ADDRESS);

    if !is_memecoin || !exchange {
        panic!("Invalid Memecoin");
    }

    let has_liquidity = call_result[6] > Felt::ZERO;
    if !has_liquidity {
        panic!("No Liquidity");
    }

    let name =
        parse_cairo_short_string(&Felt::from_bytes_be(&call_result[12].to_bytes_be())).unwrap();

    let symbol =
        parse_and_validate_short_string(&Felt::from_bytes_be(&call_result[14].to_bytes_be()))
            .unwrap();

    let total_supply = match (call_result.get(16), call_result.get(17)) {
        (Some(low), Some(high)) => parse_u256_from_felts(low, high),
        _ => "0".to_string(),
    };

    let owner = normalize_address(Felt::from_bytes_be(&call_result[19].to_bytes_be()));

    let launched_block_number = call_result[21].to_biguint();

    let team_allocation = match (call_result.get(23), call_result.get(24)) {
        (Some(low), Some(high)) => parse_u256_from_felts(low, high),
        _ => "0".to_string(),
    };

    let mut index = 28;
    let ekubo_pool_params = parse_ekubo_pool_parameters(&call_result, &mut index);
    let liquidity = Liquidity {
        launch_manager: normalize_address(Felt::from_bytes_be(&call_result[8].to_bytes_be()))
            .to_hex_string(),
        ekubo_id: EKUBO_NFT.to_string(),
        quote_token: normalize_address(Felt::from_bytes_be(&call_result[33].to_bytes_be()))
            .to_hex_string(),
        starting_tick: ekubo_pool_params.starting_price.mag.to_i64().unwrap_or(0)
            * if ekubo_pool_params.starting_price.sign {
                1
            } else {
                -1
            },
    };
    Ok(Memecoin {
        address: address.to_string(),
        name,
        symbol,
        total_supply,
        owner: owner.to_hex_string(),
        is_launched: true,
        launch: Launch {
            team_allocation,
            block_number: launched_block_number.to_u64().unwrap(),
        },
        liquidity,
    })
}

// Helper function to parse U256 from two Felt elements (high and low)
pub fn parse_u256_from_felts(low: &Felt, high: &Felt) -> String {
    u256_to_decimal_str(U256::from_words(
        low.to_u128().unwrap(),
        high.to_u128().unwrap(),
    ))
}

// Parse Ekubo Pool Parameters
fn parse_ekubo_pool_parameters(call_result: &Vec<Felt>, i: &mut usize) -> EkuboPoolParameters {
    let fee = call_result[*i].to_biguint();
    *i += 1;
    let tick_spacing = call_result[*i].to_biguint();
    *i += 1;
    println!("size: {:?}", *i);

    let starting_price_mag = call_result[*i].to_biguint();
    *i += 1;

    let starting_price_sign = call_result[*i].to_biguint().to_usize().unwrap() == 1;
    *i += 1;

    let bound = call_result[*i].to_biguint();
    *i += 1;

    EkuboPoolParameters {
        fee,
        tick_spacing,
        starting_price: StartingPrice {
            mag: starting_price_mag,
            sign: starting_price_sign,
        },
        bound,
    }
}

pub fn decode_short_string(felt: &str) -> String {
    let hex_str = felt.trim_start_matches("0x");

    // Ensure the hex string has an even length
    if hex_str.len() % 2 != 0 {
        panic!("Hex string length is not even: {}", hex_str);
    }

    // Convert hex string to bytes
    let bytes: Vec<u8> = (0..hex_str.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16).ok())
        .collect();

    // Check that we have at least one byte to decode
    if bytes.is_empty() {
        panic!("Decoded byte array is empty.");
    }

    // Attempt to decode the byte array as a UTF-8 string
    match String::from_utf8(bytes) {
        Ok(decoded_string) => decoded_string.trim_matches(char::from(0)).to_string(),
        Err(e) => {
            // If decoding fails, print the error and return the raw hex string
            eprintln!("Failed to decode bytes to string: {:?}", e);
            format!("0x{}", hex_str)
        }
    }
}

async fn multicall_contract(calls: Vec<Felt>) -> Result<Vec<Felt>, AggregateError> {
    println!("In multicall contract");
    let provider = get_provider().unwrap();

    // Make contract call with error handling
    let call_result = match provider
        .call(
            FunctionCall {
                contract_address: Felt::from_hex(MULTICALL_AGGREGATOR_ADDRESS)
                    .map_err(|e| AggregateError::ContractCall(format!("Invalid address: {}", e)))?,
                entry_point_selector: selector!("aggregate"),
                calldata: calls,
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await
    {
        std::result::Result::Ok(result) => {
            println!("Contract call successful!");
            result
        }
        Err(e) => {
            println!("Contract call failed: {:?}", e);
            return Err(AggregateError::ContractCall(format!(
                "Contract call failed: {:?}",
                e
            )));
        }
    };

    Ok(call_result)
}

pub async fn get_balance(contract_address: &str, account: &str) -> Result<String, AggregateError> {
    println!("In get balance");
    let provider = get_provider().unwrap();
    // Make contract call with error handling
    let call_result = match provider
        .call(
            FunctionCall {
                contract_address: Felt::from_hex(contract_address)
                    .map_err(|e| AggregateError::ContractCall(format!("Invalid address: {}", e)))?,
                entry_point_selector: selector!("balance_of"),
                calldata: vec![Felt::from_hex_unchecked(account)],
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await
    {
        Ok(result) => {
            println!("Contract call successful!");
            result
        }
        Err(e) => {
            println!("Contract call failed: {:?}", e);
            return Err(AggregateError::ContractCall(format!(
                "Contract call failed: {:?}",
                e
            )));
        }
    };

    let balance = match (call_result.get(0), call_result.get(1)) {
        (Some(low), Some(high)) => parse_u256_from_felts(low, high),
        _ => "0".to_string(),
    };

    Ok(balance)
}

pub async fn validate_memecoins(addresses: Vec<&str>) -> Result<Vec<&str>, Error> {
    println!("In validate memecall");
    let calls = generate_validate_calls(addresses.clone());
    let call_result = multicall_contract(calls).await.unwrap();
    let mut memecoin_addresses: Vec<&str> = Vec::new();
    // Iterate over each data item in call_result (starting from index 2)
    for (index, data) in call_result
        .iter()
        .skip(2)
        .take(addresses.len() - 2)
        .enumerate()
    {
        let is_memecoin = *data > Felt::ZERO;

        if is_memecoin {
            memecoin_addresses.push(addresses[index]);
        }
    }
    Ok(memecoin_addresses)
}

fn generate_validate_calls(addresses: Vec<&str>) -> Vec<Felt> {
    println!("In generate validate calls");
    let mut calls: Vec<Felt> = vec![Felt::from(addresses.len())];
    let factory_address = MEMECOIN_FACTORY_ADDRESS;
    for address in addresses {
        calls.push(Felt::from_hex_unchecked(factory_address));
        calls.push(get_selector_from_name("is_memecoin").unwrap());
        calls.push(Felt::ONE);
        calls.push(Felt::from_hex_unchecked(address));
    }
    calls
}
