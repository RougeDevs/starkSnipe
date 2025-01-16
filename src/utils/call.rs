use core::time;
use std::str::FromStr;
use std::str::{from_utf8};
use std::time::Instant;
use apibara_core::starknet::v1alpha2::FieldElement;
use hex::decode;
use kanshi::utils::conversions::{field_to_hex_string, field_to_string};
use serde::{Deserialize, Serialize};
use starknet::core::types::{BlockId, BlockTag, FunctionCall, U256};
use starknet::core::utils::{cairo_short_string_to_felt, get_selector_from_name, normalize_address, parse_cairo_short_string};
use starknet::macros::selector;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, Provider, ProviderError};
use starknet_core::types::Felt;
use url::Url;
use num_bigint::{BigInt, BigUint};
use num_traits::cast::ToPrimitive;


use crate::constant::constants::{selector_to_str, Selector};
use crate::utils::event_parser::{parse_and_validate_short_string, u256_to_decimal_str};

const MULTICALL_AGGREGATOR_ADDRESS: &str = "0x01a33330996310a1e3fa1df5b16c1e07f0491fdd20c441126e02613b948f0225";
const MEMECOIN_FACTORY_ADDRESS: &str = "0x01a46467a9246f45c8c340f1f155266a26a71c07bd55d36e8d1c7d0d438a2dbc";
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memecoin {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub total_supply: String,
    pub owner: String,
    pub is_launched: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch: Option<Launch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidity: Option<Liquidity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Launch {
    pub team_allocation: String,
    pub block_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Liquidity {
    pub launch_manager: String,
    pub ekubo_id: String,
    pub quote_token: String,
    pub starting_tick: i64,
}

#[derive(Debug)]
struct EkuboPoolParameters {
    fee: BigUint,
    tick_spacing: BigUint,
    starting_price: StartingPrice,
    bound: BigUint,
}

#[derive(Debug)]
struct StartingPrice {
    mag: BigUint,
    sign: bool,
}

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

const EKUBO_NFT: &str = "0xbf8a9";

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

pub async fn get_aggregate_call_data(address: &str) -> Result<Vec<String>, AggregateError> {
    println!("Entering aggregate call data");
    
    // Create provider with error handling
    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse("https://starknet-mainnet.public.blastapi.io/rpc/v0_7")
            .map_err(AggregateError::Url)?
    ));
    
    println!("provider {:?}", provider);
    let calls = generate_calls(address);
    println!("calls {:?}", calls);
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
        .await {
            Ok(result) => {
                println!("Contract call successful!");
                result
            }
            Err(e) => {
                println!("Contract call failed: {:?}", e);
                return Err(AggregateError::ContractCall(format!("Contract call failed: {:?}", e)));
            }
        };
    println!("Call result \n {:?}", call_result);

    // Parse results with error handling
    let parsed_result = parse_call_result(call_result);
    
    Ok(parsed_result)
}


fn generate_calls(address: &str) -> Vec<starknet_core::types::Felt>{
    println!("entering generate_calls");
    let start_time = Instant::now();
    let mut calls: Vec<Felt> = vec![Felt::from(8)];
    println!("calls {:?}", calls);
    // Check if address is a memecoin
    calls.push(Felt::from_hex_unchecked(MEMECOIN_FACTORY_ADDRESS));
    calls.push(get_selector_from_name(&selector_to_str(Selector::IsMemecoin)).unwrap());
    calls.push(Felt::ONE);
    calls.push(Felt::from_hex_unchecked(address));
    println!("calls 148 {:?}", calls);
    // Check on address for memecoin name
    calls.push(Felt::from_hex_unchecked(address));
    calls.push(get_selector_from_name(&selector_to_str(Selector::Name)).unwrap());
    calls.push(Felt::ZERO);
    println!("calls 153 {:?}", calls);

    // Check on address for memecoin symbol
    calls.push(Felt::from_hex_unchecked(address));
    calls.push(get_selector_from_name(&selector_to_str(Selector::Symbol)).unwrap());
    calls.push(Felt::ZERO);
    println!("calls 159 {:?}", calls);

    // Check on address for total supply
    calls.push(Felt::from_hex_unchecked(address));
    calls.push(get_selector_from_name(&selector_to_str(Selector::TotalSupply)).unwrap());
    calls.push(Felt::ZERO);

    // Check on address for owner
    calls.push(Felt::from_hex_unchecked(address));
    calls.push(get_selector_from_name(&selector_to_str(Selector::Owner)).unwrap());
    calls.push(Felt::ZERO);

    // Check on address for is_launched
    calls.push(Felt::from_hex_unchecked(address));
    calls.push(get_selector_from_name(&selector_to_str(Selector::LaunchedAtBlockNumber)).unwrap());
    calls.push(Felt::ZERO);

    println!("time: {:?}", start_time.elapsed());
    // Check on address for get_team_allocation
    calls.push(Felt::from_hex_unchecked(address));
    calls.push(get_selector_from_name(&selector_to_str(Selector::GetTeamAllocation)).unwrap());
    calls.push(Felt::ZERO);

    // Check onlaunch with liquidatiy parameters
    calls.push(Felt::from_hex_unchecked(address));
    calls.push(get_selector_from_name(&selector_to_str(Selector::LaunchedWithLiquidityParameters)).unwrap());
    calls.push(Felt::ZERO);
    println!("calls: \n{:?}",calls);
    calls
}

fn parse_call_result(call_result: Vec<Felt>) -> Vec<String> {
    println!("call results length: {:?}", call_result.len());
    let mut i = 1; // Skip block_number
    let total_length = call_result[i].to_bytes_be();
    i += 1;
    
    let mut responses = vec![];

    // Safely handle name parsing
    if let Ok(name) = parse_cairo_short_string(&Felt::from_bytes_be(&call_result[5].to_bytes_be())) {
        responses.push(format!("Name: {}", name));
    } else {
        responses.push("Name: <invalid>".to_string());
    }

    // Symbol parsing with safe conversion
    if let Ok(symbol) = parse_and_validate_short_string(&Felt::from_bytes_be(
        &call_result[7].to_bytes_be()
    )) {
        responses.push(format!("Symbol: {}", symbol));
    } else {
        responses.push("Symbol: <invalid>".to_string());
    }
    // let symbol = field_to_string(&call_result[7]);
    // responses.push(format!("Symbol: {}", symbol));

    // Total Supply parsing with safe U256 construction
    let total_supply = match (call_result.get(9), call_result.get(10)) {
        (Some(high), Some(low)) => {
            let high_bytes = high.to_bytes_be();
            let low_bytes = low.to_bytes_be();
            let high_u128 = u128::from_be_bytes(high_bytes[16..32].try_into().unwrap_or([0; 16]));
            let low_u128 = u128::from_be_bytes(low_bytes[16..32].try_into().unwrap_or([0; 16]));
            u256_to_decimal_str(U256::from_words(high_u128, low_u128))
        }
        _ => "0".to_string()
    };
    responses.push(format!("Total Supply: {}", total_supply));

    // Owner address with safe conversion
    // let owner = field_to_hex_string(&call_result[12]);
    let owner = normalize_address(Felt::from_bytes_be(
        &call_result[12].to_bytes_be(),
    ));
    responses.push(format!("Owner: {}", owner));
    

    // Launch block number with safe conversion
    let launched_block_number = call_result[14].to_bytes_be();
    responses.push(format!("Launched Block Number: 0x{}", hex::encode(launched_block_number)));

    // Team allocation with safe U256 construction
    let team_allocation = match (call_result.get(16), call_result.get(17)) {
        (Some(high), Some(low)) => {
            let high_bytes = high.to_bytes_be();
            let low_bytes = low.to_bytes_be();
            let high_u128 = u128::from_be_bytes(high_bytes[16..32].try_into().unwrap_or([0; 16]));
            let low_u128 = u128::from_be_bytes(low_bytes[16..32].try_into().unwrap_or([0; 16]));
            u256_to_decimal_str(U256::from_words(high_u128, low_u128))
        }
        _ => "0".to_string()
    };
    responses.push(format!("Team Allocation: {}", team_allocation));

    println!("Responses: {:?}", responses);
    responses
}

// Helper function to safely convert field elements to hex strings
pub fn safe_field_to_hex(felt: &Felt) -> String {
    format!("0x{}", hex::encode(felt.to_bytes_be()))
}

// Updated combine_biguints function with overflow protection
fn combine_biguints(high: BigUint, low: BigUint) -> BigUint {
    let shift_amount = 128u32;
    let two = BigUint::from(2u32);
    
    // Check if shift would overflow
    if high.bits() > 128 {
        return low; // Return just the low bits if high bits would overflow
    }
    
    high * two.pow(shift_amount) + low
}

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



pub fn get_checksum_address(address: &str) -> String {
    address.to_string()
}