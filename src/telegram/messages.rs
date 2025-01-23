use serde_json::json;

use crate::{telegram::types::common::BroadcastEvent, utils::{info_aggregator::{aggregate_info, get_account_holding_info, get_account_holdings}, types::common::MemecoinInfo}};

use super::utils::{
    calculate_team_allocation, format_large_number, format_number, format_percentage, format_price, format_short_address, is_valid_starknet_address
};

pub fn generate_broadcast_event(event_data: MemecoinInfo) -> String {
    let payload = BroadcastEvent {
        name: event_data.name,
        symbol: event_data.symbol,
        address: event_data.address,
        market_cap: format_price(event_data.market_cap),
        total_supply: format_number(&format_large_number(&event_data.total_supply).unwrap())
            .unwrap(),
        liquidity: format!(
            "{:.2}",
            event_data.usd_dex_liquidity.parse::<f64>().unwrap()
        ),
        team_allocation: format_percentage(calculate_team_allocation(
            event_data.total_supply,
            event_data.team_allocation,
        )),
    };

    format!(
        "🚨 ====== *FRESH LAUNCH ALERT* ====== 🚨\n\n\
                *{}* ({}) has landed on Starknet!\n\n\
                *Address:* {}\n\
                *Starting MCAP:* ${}\n\
                *Supply:* {}\n\
                *Liquidity:* ${}\n\
                *Team:* {}%\n\
                ⚡️ *GET IN NOW*\n\n\
                #Starknet #Memecoin #{}",
        payload.name,
        payload.symbol,
        payload.address,
        payload.market_cap,
        payload.total_supply,
        payload.liquidity,
        payload.team_allocation,
        payload.symbol
    )
}

pub fn create_launch_keyboard(
    dex_url: &str,
    contract_address: &str,
    token_symbol: &str,
) -> serde_json::Value {
    json!({
        "inline_keyboard": [
            [
                {
                    "text": "🚀 Buy $10",
                    "url": format!("{}?token={}&amount=10&symbol={}",
                        dex_url, contract_address, token_symbol)
                },
                {
                    "text": "🚀 Buy $50",
                    "url": format!("{}?token={}&amount=50&symbol={}",
                        dex_url, contract_address, token_symbol)
                },
                {
                    "text": "🚀 Buy $100",
                    "url": format!("{}?token={}&amount=100&symbol={}",
                        dex_url, contract_address, token_symbol)
                }
            ],
            [
                {
                    "text": "💰 Custom Amount",
                    "url": format!("{}?token={}",
                        dex_url, contract_address)
                }
            ]
        ]
    })
}

pub async fn handle_spot_command(wallet_address: String, token_address: String, dex_url: &str) -> String {
    if !is_valid_starknet_address(&wallet_address) || !is_valid_starknet_address(&token_address) {
        "❌ Malformed address".to_string()
    } else {
        match get_account_holding_info(&wallet_address, &token_address).await {
            Ok(info) => {
                format!(
                    "📊 ====== *TOKEN SPOT* ====== 📊\n\n\
                    *Wallet:* {}\n\
                    *Token:* ${}\n\n\
                    *POSITION*\n\
                    *Balance:* {}\n\
                    *Worth:* ${}\n\n\
                    *ACTIONS*\n\
                    ⚡️ *Trade Now:* {}",
                    format_short_address(&wallet_address),
                    info.coin_info.symbol,
                    format_large_number(&info.account_balance).unwrap(),
                    info.usd_value,
                    dex_url
                )
            }
            Err(e) => {
                format!(
                    "❌ Error fetching token info: {}",
                    if e.to_string().contains("parse") {
                        "Invalid token data format"
                    } else if e.to_string().contains("aggregate_info") {
                        "Failed to fetch token information"
                    } else if e.to_string().contains("get_balance") {
                        "Failed to fetch account balance"
                    } else {
                        "Unexpected error occurred"
                    }
                )
            }
        }
    }
}

pub async fn handle_peek_command(wallet_address: String) -> String {
    if !is_valid_starknet_address(&wallet_address) {
        let error_message = format!("malformed address");
        error_message
    } else {
        match get_account_holdings(&wallet_address).await {
            Ok(holdings) => {
                let message = format!("
                        💼 ====== *BAG CHECK* ====== 💼\n\n\
                        👛 *Wallet:* \n{}\n\n\
                        💼 *PORTFOLIO*\n\
                        🎯 *Total Memecoins:* {}\n\n\
                        💡 *TIP:* Check token position\n\
                        *Use: /spot <wallet> <token>*
                ",
                    holdings.account_address,
                    holdings.total_tokens
                );
                message
            }
            Err(e) => {
                let error_message = format!("Error peeking into wallet ⁉️");
                error_message
            }
        }
    }
}

pub async fn handle_sniq_command(token_address: String, dex_url: &str, explorer_url: &str) -> String {
    if !is_valid_starknet_address(&token_address) {
        let error_message = format!("malformed address");
        error_message
    } else {

        match aggregate_info(&token_address).await {
            Ok(response) => {
                let message = format!("
                             ⚡ ====== *SNIQ RADAR* ======⚡\n\
                        \n\
                        *Token:* ${}\n\
                        *Name:* {}\n\
                        *Contract:* {}\n\n\
                        📊 *METRICS*\n\
                        💰 *Price:* ${}\n\
                        📈 *MCap:* ${}\n\
                        💫 *Supply:* ${}\n\
                        👥 *Holders:* {}\n\
                        💧 *LP:* ${}\n\n\
                        🛡 *SECURITY CHECK*\n\
                        🔒 *LP Status:* Locked Forever\n\
                        ✅ *Contract:* Verified\n\n\
                        🔗 *QUICK LINKS*\n\
                        🎯 *Trade:* {}\n\
                        🔍 *Explorer:* {}\n\
                        ",
                        response.0.symbol,
                        response.0.name,
                        response.0.address,
                        response.0.price,
                        format_number(&response.0.market_cap).unwrap(),
                        format_number(&format_large_number(&response.0.total_supply).unwrap()).unwrap(),
                        response.1.category,
                        format_number(&response.0.usd_dex_liquidity).unwrap(),
                        dex_url,
                        format!("{}/{}",explorer_url, response.0.address )
                    );
                    message
            },
            Err(error) => {
                let error_message = format!("Error fetching token details ⁉️");
                error_message
            }
        }
    }
}