use std::fmt;

use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct EkuboMemecoin {
    pub liquidity: Liquidity,
    pub launch: Launch,
    pub total_supply: BigUint,
}

#[derive(Debug, Clone)]
pub struct EkuboLiquidityLockPosition {
    pub unlock_time: u64,
    pub owner: String,
    pub pool_key: PoolKey,
    pub bounds: Bounds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memecoin {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub total_supply: String,
    pub owner: String,
    pub is_launched: bool,
    pub launch: Launch,
    pub liquidity: Liquidity,
}

impl Default for Memecoin {
    fn default() -> Self {
        Self {
            address: Default::default(),
            name: Default::default(),
            symbol: Default::default(),
            total_supply: Default::default(),
            owner: Default::default(),
            is_launched: Default::default(),
            launch: Default::default(),
            liquidity: Default::default(),
        }
    }
}

impl fmt::Display for Memecoin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Memecoin {{ address: {}, name: {}, symbol: {}, total_supply: {}, owner: {}, is_launched: {}, launch: {:?}, liquidity: {:?} }}",
            self.address, self.name, self.symbol, self.total_supply, self.owner, self.is_launched, self.launch, self.liquidity)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Launch {
    pub team_allocation: String,
    pub block_number: u64,
}

impl Default for Launch {
    fn default() -> Self {
        Self {
            team_allocation: Default::default(),
            block_number: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Liquidity {
    pub launch_manager: String,
    pub ekubo_id: String,
    pub quote_token: String,
    pub starting_tick: i64,
}

impl Default for Liquidity {
    fn default() -> Self {
        Self {
            launch_manager: Default::default(),
            ekubo_id: Default::default(),
            quote_token: Default::default(),
            starting_tick: Default::default(),
        }
    }
}

impl fmt::Display for Liquidity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Liquidity {{ launch_manager: {}, ekubo_id: {}, quote_token: {}, starting_tick: {} }}",
            self.launch_manager, self.ekubo_id, self.quote_token, self.starting_tick
        )
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct PoolKey {
    pub token0: String,
    pub token1: String,
    pub fee: String,
    pub tick_spacing: String,
    pub extension: String,
}

#[derive(Debug)]
pub struct EkuboPoolParameters {
    pub fee: BigUint,
    pub tick_spacing: BigUint,
    pub starting_price: StartingPrice,
    pub bound: BigUint,
}

#[derive(Debug)]
pub struct StartingPrice {
    pub mag: BigUint,
    pub sign: bool,
}

#[derive(Debug, Clone)]
pub struct Bounds {
    pub lower: Bound,
    pub upper: Bound,
}

#[derive(Debug, Clone)]
pub struct Bound {
    pub mag: String,
    pub sign: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PoolKeyResponse {
    pub token0: String,
    pub token1: String,
    pub fee: String,
    pub tick_spacing: u64,
    pub extension: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RouteResponse {
    pub pool_key: PoolKeyResponse,
    pub sqrt_ratio_limit: String,
    pub skip_ahead: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SplitResponse {
    pub amount: String,
    #[serde(rename = "specifiedAmount")]
    pub specified_amount: String,
    pub route: Vec<RouteResponse>,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct QuoteResponseApi {
    pub total: String,
    pub splits: Vec<SplitResponse>,
}
