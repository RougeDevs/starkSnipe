use std::collections::HashSet;

use anyhow::Ok;

use super::call::{get_aggregate_call_data, get_balance, validate_memecoins};
use super::market_cap::calculate_market_cap;
use super::types::common::{
    FilteredTokenData, HolderApiResponse, Holders, HoldingApiResponse, MemecoinInfo,
    TokenCategoryResponse, TokenHoldings, UserTokenInfo,
};
use super::types::ekubo::Memecoin;

async fn fetch_holders_data(token_address: &str) -> Result<TokenCategoryResponse, anyhow::Error> {
    let explorer_env = std::env::var("EXPLORER_API").expect("EXPLORER_API must be set.");

    let url = format!(
        "{}/{}/holders?ps=100&type=erc20",
        explorer_env, token_address
    );

    let response = reqwest::get(&url)
        .await?
        .json::<HolderApiResponse>()
        .await?;

    let filtered_items: Vec<Holders> = response
        .items
        .into_iter()
        .filter(|holder| {
            !matches!(
                holder.contractAlias.as_deref(),
                Some("Unruggable.meme") | Some("Ekubo: Core")
            )
        })
        .collect();

    let category = if response.hasMore {
        format!("🌑 *>100 hodlers* — *Moon phase incoming!*")
    } else {
        match filtered_items.len() {
            0..=9 => format!("🌱 *<10* — *Early bird special!*"),
            10..=19 => format!("🚀 *>10* — *FOMO vibes!*"),
            20..=49 => format!("🔥 *>20* — *It’s heating up! 🔥*"),
            _ => format!("💥 *>50* — *Time to jump in!*"),
        }
    };

    let result = TokenCategoryResponse {
        token_address: token_address.to_string(),
        category: category.to_string(),
    };

    Ok(result)
}

async fn is_valid_account(account: &str) -> Result<bool, anyhow::Error> {
    let explorer_env = std::env::var("EXPLORER_API").expect("EXPLORER_API must be set.");
    let url = format!("{}/{}/", explorer_env, account);
    let response = reqwest::get(&url)
        .await?
        .json::<serde_json::Value>()
        .await?;

    Ok(response
        .get("isAccount")
        .and_then(|v| v.as_bool())
        .unwrap_or(false))
}

async fn fetch_account_holdings(account: &str) -> Result<Vec<FilteredTokenData>, anyhow::Error> {
    let is_valid = is_valid_account(account).await?;
    if !is_valid {
        panic!("{} is not a valid account", account);
    }

    let explorer_env = std::env::var("EXPLORER_API").expect("EXPLORER_API must be set.");
    let url = format!("{}/{}/token-balances", explorer_env, account);

    // Send the request and fetch the response
    let response = reqwest::get(&url)
        .await?
        .json::<HoldingApiResponse>()
        .await?;

    // Filter and parse the response to get only tokens with 18 decimals
    let filtered_tokens = parse_token_data(&response);

    Ok(filtered_tokens)
}

fn parse_token_data(api_response: &HoldingApiResponse) -> Vec<FilteredTokenData> {
    let mut filtered_tokens = Vec::new();

    for token in &api_response.erc20TokenBalances {
        // Convert decimals from hex to u32 and check if it's 18
        let decimals = u32::from_str_radix(&token.decimals[2..], 16).unwrap_or(0);

        // Filter tokens with exactly 18 decimals
        if decimals == 18 {
            filtered_tokens.push(FilteredTokenData {
                name: token.name.clone(),
                address: token.address.clone(),
                balance: token.balance.clone(),
                formatted_balance: token.formattedBalance.clone(),
                symbol: token.symbol.clone(),
            });
        }
    }

    filtered_tokens
}

pub async fn aggregate_info(
    token_address: &str,
) -> Result<(MemecoinInfo, TokenCategoryResponse), anyhow::Error> {
    let ekubo_core = std::env::var("EKUBO_CORE_ADDRESS").expect("EKUBO_CORE_ADDRESS must be set.");
    let aggregated_data: Memecoin = get_aggregate_call_data(&token_address).await?;
    let data = calculate_market_cap(&aggregated_data.total_supply, &aggregated_data.symbol).await;
    let mut price = String::new();
    let mut market_cap = String::new();
    if data.is_ok() {
        (price, market_cap) = data.unwrap();
    }
    let holders_data: TokenCategoryResponse = fetch_holders_data(&token_address).await?;
    let ekubo_core_balance = get_balance(&token_address, &ekubo_core).await?;
    let ekubo_core_balance_f64: f64 = ekubo_core_balance.parse()?;
    let price_f64: f64 = price.parse()?;
    let liquidity = (ekubo_core_balance_f64 * price_f64).to_string();
    Ok((
        MemecoinInfo {
            address: token_address.to_string(),
            name: aggregated_data.name,
            symbol: aggregated_data.symbol,
            total_supply: aggregated_data.total_supply,
            owner: aggregated_data.owner,
            team_allocation: aggregated_data.launch.team_allocation,
            price,
            market_cap,
            usd_dex_liquidity: liquidity,
        },
        holders_data,
    ))
}

pub async fn get_account_holdings(account: &str) -> Result<TokenHoldings, anyhow::Error> {
    let token_data: Vec<FilteredTokenData> = fetch_account_holdings(account).await?;
    let addresses: Vec<&str> = token_data
        .iter()
        .map(|token| token.address.as_str())
        .collect();
    let valid_addresses = validate_memecoins(addresses).await.unwrap();
    let valid_address_set: HashSet<String> =
        valid_addresses.into_iter().map(|s| s.to_string()).collect();

    // This filtered_tokens can be utilised further
    let filtered_tokens: Vec<FilteredTokenData> = token_data
        .into_iter()
        .filter(|token| valid_address_set.contains(&token.address))
        .collect();
    Ok(TokenHoldings {
        account_address: account.to_string(),
        total_tokens: filtered_tokens.len().to_string(),
    })
}

pub async fn get_account_holding_info(
    account: &str,
    token_address: &str,
) -> Result<UserTokenInfo, anyhow::Error> {
    let coin_info = aggregate_info(token_address).await?;
    let account_balance = get_balance(&token_address, account).await?;
    let account_balance_f64: f64 = account_balance.parse()?;
    let price_f64: f64 = coin_info.0.price.parse()?;
    let usd_value = account_balance_f64 * price_f64;
    let usd_value_str = format!("{:.2}", usd_value);
    Ok(UserTokenInfo {
        coin_info: coin_info.0,
        account_balance,
        usd_value: usd_value_str,
    })
}
