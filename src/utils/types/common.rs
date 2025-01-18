use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemecoinInfo {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub total_supply: String,
    pub owner: String,
    pub team_allocation: String,
    pub price: String,
    pub market_cap: String,
    pub usd_dex_liquidity: String,
}

#[derive(Deserialize, Debug)]
pub struct Holders {
    pub holder: String,
    pub balance: String,
    pub lastTransferTime: u64,
    pub decimals: String,
    pub balanceSeparated: String,
    pub contractAlias: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct HolderApiResponse {
    pub items: Vec<Holders>,
    pub lastPage: u32,
    pub hasMore: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenCategoryResponse {
    pub token_address: String,
    pub category: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InfoResponse {
    pub coin_info: MemecoinInfo,
    pub holders_data: TokenCategoryResponse,
}
#[derive(Debug, Deserialize, Clone)]
pub struct TokenHoldings {
    pub account_address: String,
    pub total_tokens: String,
}

#[derive(Deserialize, Debug)]
pub struct HoldingApiResponse {
    pub erc20TokenBalances: Vec<TokenBalance>,
    pub verfiedTokensCount: u32,
    pub totalTokensCount: u32,
    pub totalUsdValue: String,
}

#[derive(Deserialize, Debug)]
pub struct TokenBalance {
    pub name: String,
    pub address: String,
    pub balance: String,
    pub usdBalance: Option<String>,          // Could be null
    pub usdFormattedBalance: Option<String>, // Could be null
    pub decimals: String,
    pub symbol: String,
    pub formattedBalance: String,
    pub iconName: String,
    pub isVerified: bool,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct FilteredTokenData {
    pub name: String,
    pub address: String,
    pub balance: String,
    pub formatted_balance: String,
    pub symbol: String,
}

#[derive(Debug)]
pub struct UserTokenInfo {
    pub coin_info: MemecoinInfo,
    pub account_balance: String,
    pub usd_value: String,
}
