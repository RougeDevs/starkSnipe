use anyhow::Context;
use starknet::core::types::{Felt, U256};
use starknet::core::utils::{normalize_address, parse_cairo_short_string};

use super::call::get_aggregate_call_data;
pub trait FromFieldBytes: Sized {
    fn from_field_bytes(bytes: [u8; 32]) -> Self;
}

impl FromFieldBytes for u8 {
    fn from_field_bytes(bytes: [u8; 32]) -> Self {
        bytes[31]
    }
}

impl FromFieldBytes for u16 {
    fn from_field_bytes(bytes: [u8; 32]) -> Self {
        let last_two_bytes: [u8; 2] = bytes[30..32]
            .try_into()
            .expect("Slice with incorrect length");
        u16::from_be_bytes(last_two_bytes)
    }
}

impl FromFieldBytes for u64 {
    fn from_field_bytes(bytes: [u8; 32]) -> Self {
        let last_eight_bytes: [u8; 8] = bytes[24..32]
            .try_into()
            .expect("Slice with incorrect length");
        u64::from_be_bytes(last_eight_bytes)
    }
}

impl FromFieldBytes for u128 {
    fn from_field_bytes(bytes: [u8; 32]) -> Self {
        let last_sixteen_bytes: [u8; 16] = bytes[16..32]
            .try_into()
            .expect("Slice with incorrect length");
        u128::from_be_bytes(last_sixteen_bytes)
    }
}

pub fn u256_to_decimal_str(value: U256) -> String {
    format!("{}", value)
}

pub fn parse_and_validate_short_string(felt: &Felt) -> anyhow::Result<String> {
    let result = parse_cairo_short_string(felt)?;

    if !result
        .chars()
        .all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace())
    {
        return Ok(felt.to_string());
    }
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct CreationEvent {
    #[allow(unused)]
    pub owner: Felt,
    #[allow(unused)]
    pub name: String,
    #[allow(unused)]
    pub symbol: String,
    #[allow(unused)]
    pub initial_supply: String,
    #[allow(unused)]
    pub memecoin_address: Felt,
}

#[derive(Debug, Clone)]
pub struct LaunchEvent {
    #[allow(unused)]
    pub memecoin_address: Felt,
    #[allow(unused)]
    pub quote_token: Felt,
    #[allow(unused)]
    pub exchange_name: String,
}

pub trait FromStarknetEventData: Sized {
    fn from_starknet_event_data(data: Vec<Felt>) -> anyhow::Result<Self>;
}

impl FromStarknetEventData for CreationEvent {
    fn from_starknet_event_data(data: Vec<Felt>) -> Result<Self, anyhow::Error> {
        let mut data = data.iter();

        let owner = normalize_address(Felt::from_bytes_be(
            &data.next().context("Missing owner")?.to_bytes_be(),
        ));
        let name: String = parse_cairo_short_string(&Felt::from_bytes_be(
            &data.next().context("Missing name")?.to_bytes_be(),
        ))?;
        let symbol: String = parse_and_validate_short_string(&Felt::from_bytes_be(
            &data.next().context("Missing symbol")?.to_bytes_be(),
        ))?;
        let initial_supply = u256_to_decimal_str(U256::from_words(
            u128::from_field_bytes(
                data.next()
                    .context("Missing initial_supply low")?
                    .to_bytes_be(),
            ),
            u128::from_field_bytes(
                data.next()
                    .context("Missing initial_supply high")?
                    .to_bytes_be(),
            ),
        ));
        let memecoin_address = normalize_address(Felt::from_bytes_be(
            &data
                .next()
                .context("Missing memecoin_address")?
                .to_bytes_be(),
        ));

        let creation_data = Self {
            owner,
            name,
            symbol,
            initial_supply,
            memecoin_address,
        };
        Ok(creation_data)
    }
}

impl FromStarknetEventData for LaunchEvent {
    fn from_starknet_event_data(data: Vec<Felt>) -> Result<Self, anyhow::Error> {
        let mut data = data.iter();

        let memecoin_address = normalize_address(Felt::from_bytes_be(
            &data
                .next()
                .context("Missing memecoin_address")?
                .to_bytes_be(),
        ));
        let quote_token = normalize_address(Felt::from_bytes_be(
            &data.next().context("Missing quote_token")?.to_bytes_be(),
        ));
        let exchange_name: String = parse_cairo_short_string(&Felt::from_bytes_be(
            &data.next().context("Missing exchange_name")?.to_bytes_be(),
        ))?;

        let launch_data = Self {
            memecoin_address,
            quote_token,
            exchange_name,
        };
        Ok(launch_data)
    }
}
