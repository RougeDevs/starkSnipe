use lazy_static::lazy_static;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selector {
    IsMemecoin,
    Aggregate,
    Name,
    Symbol,
    IsLaunched,
    GetTeamAllocation,
    TotalSupply,
    Owner,
    LockedLiquidity,
    ExchangeAddress,
    Approve,
    GetRemainingTime,
    LaunchedWithLiquidityParameters,
    GetLockDetails,
    LaunchedAtBlockNumber,
    GetReserves,
    LiquidityPositionDetails,
    BalanceOfCamel,
    BalanceOf,
    Transfer,
    GetTokenInfos,
    ClearMinimum,
    Clear,
    MultihopSwap,
    MultiMultihopSwap,
    GetBalances,
}

// Returns the string representation of the selector
pub fn selector_to_str(selector: Selector) -> &'static str {
    match selector {
        Selector::IsMemecoin => "is_memecoin",
        Selector::Aggregate => "aggregate",
        Selector::Name => "name",
        Selector::Symbol => "symbol",
        Selector::IsLaunched => "is_launched",
        Selector::GetTeamAllocation => "get_team_allocation",
        Selector::TotalSupply => "total_supply",
        Selector::Owner => "owner",
        Selector::LockedLiquidity => "locked_liquidity",
        Selector::ExchangeAddress => "exchange_address",
        Selector::Approve => "approve",
        Selector::GetRemainingTime => "get_remaining_time",
        Selector::LaunchedWithLiquidityParameters => "launched_with_liquidity_parameters",
        Selector::GetLockDetails => "get_lock_details",
        Selector::LaunchedAtBlockNumber => "launched_at_block_number",
        Selector::GetReserves => "get_reserves",
        Selector::LiquidityPositionDetails => "liquidity_position_details",
        Selector::BalanceOfCamel => "balanceOf",
        Selector::BalanceOf => "balance_of",
        Selector::Transfer => "transfer",
        Selector::GetTokenInfos => "get_token_info",
        Selector::ClearMinimum => "clear_minimum",
        Selector::Clear => "clear",
        Selector::MultihopSwap => "multihop_swap",
        Selector::MultiMultihopSwap => "multi_multihop_swap",
        Selector::GetBalances => "get_balances",
    }
}

// Define the TokenSymbol enum to represent different token symbols.
#[derive(Debug, Clone)]
pub enum TokenSymbol {
    ETH,
    USDC,
    STRK,
    USDT,
    WBTC,
    DAI,
}

// Define the Token struct to hold token data.
#[derive(Debug, Clone)]
pub struct Token {
    pub address: &'static str,
    pub symbol: TokenSymbol,
    pub decimals: u8,
    pub camel_cased: bool,
    pub usdc_pair: &'static str,
}

// Declare the tokens as constants.
pub const ETHER: Token = Token {
    address: "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
    symbol: TokenSymbol::ETH,
    decimals: 18,
    camel_cased: true,
    usdc_pair: "0x04d0390b777b424e43839cd1e744799f3de6c176c7e32c1812a41dbd9c19db6a",
};

pub const USDC: Token = Token {
    address: "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
    symbol: TokenSymbol::USDC,
    decimals: 6,
    camel_cased: true,
    usdc_pair: "",
};

pub const STRK: Token = Token {
    address: "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d",
    symbol: TokenSymbol::STRK,
    decimals: 18,
    camel_cased: true,
    usdc_pair: "0x5726725e9507c3586cc0516449e2c74d9b201ab2747752bb0251aaa263c9a26",
};

pub const USDT: Token = Token {
    address: "0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8",
    symbol: TokenSymbol::USDT,
    decimals: 6,
    camel_cased: true,
    usdc_pair: "0x5801bdad32f343035fb242e98d1e9371ae85bc1543962fedea16c59b35bd19b",
};

lazy_static! {
    pub static ref QUOTE_TOKENS: HashMap<String, Token> = {
        let mut m = HashMap::new();
        m.insert(get_checksum_address(ETHER.address), ETHER);
        m.insert(get_checksum_address(STRK.address), USDC);
        m.insert(get_checksum_address(USDC.address), STRK);
        m.insert(get_checksum_address(USDT.address), USDT);
        m
    };
}

pub fn get_checksum_address(address: &str) -> String {
    address.to_string()
}

pub const JEDISWAP_ETH_USDC_POOL: &str =
    "0x04d0390b777b424e43839cd1e744799f3de6c176c7e32c1812a41dbd9c19db6a";
pub const DECIMALS: u32 = 18;
pub const LIQUIDITY_LOCK_FOREVER_TIMESTAMP: u64 = 9999999999; // 20/11/2286
pub const EKUBO_TICK_SIZE: f64 = 1.000001;
pub const MULTICALL_AGGREGATOR_ADDRESS: &str =
    "0x01a33330996310a1e3fa1df5b16c1e07f0491fdd20c441126e02613b948f0225";
pub const MEMECOIN_FACTORY_ADDRESS: &str =
    "0x01a46467a9246f45c8c340f1f155266a26a71c07bd55d36e8d1c7d0d438a2dbc";
pub const EXCHANGE_ADDRESS: &str =
    "0x2bd1cdd5f7f17726ae221845afd9580278eebc732bc136fe59d5d94365effd5";
