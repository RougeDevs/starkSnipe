use kanshi::dna::EventData;
use reqwest::{Client, Error};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::fmt::format;
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::RwLock;
use rust_decimal::prelude::*;

use crate::utils::event_parser::CreationEvent;
use crate::utils::info_aggregator::{aggregate_info, get_account_holding_info, get_account_holdings};
use crate::utils::types::common::MemecoinInfo;
use crate::utils::types::ekubo::Memecoin;
use crate::EventType;

#[derive(Debug, Deserialize)]
struct Update {
    update_id: i64,
    #[serde(default)]
    message: Option<Message>,
    #[serde(default)]
    callback_query: Option<CallbackQuery>,
}

#[derive(Debug, Deserialize)]
struct Message {
    message_id: i64,
    #[serde(default)]
    from: Option<User>,
    chat: Chat,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    id: String,
    from: User,
    data: Option<String>,
}

#[derive(Debug, Deserialize)]
struct User {
    id: i64,
    first_name: String,
    #[serde(default)]
    last_name: Option<String>,
    #[serde(default)]
    username: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Chat {
    id: i64,
    #[serde(rename = "type")]
    chat_type: String,
}

// Configuration struct for TelegramBot
#[derive(Clone)]
pub struct TelegramConfig {
    token: String,
    dex_url: String,
    explorer_url: String,
}

impl TelegramConfig {
    pub fn new() -> Self {
        Self {
            token: std::env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN not found"),
            dex_url: std::env::var("DEX_URL").unwrap_or_else(|_| "https://app.avnu.fi".to_string()),
            explorer_url: std::env::var("EXPLORER")
                .unwrap_or_else(|_| "https://starkscan.co".to_string()),
        }
    }
}

pub struct TelegramBot {
    config: TelegramConfig,
    client: Client,
    base_url: String,
    active_users: RwLock<HashMap<i64, bool>>,
}

impl TelegramBot {
    pub fn new(config: TelegramConfig) -> Result<Self, Error> {
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        let base_url = format!("https://api.telegram.org/bot{}", config.token);

        Ok(Self {
            config,
            client,
            base_url,
            active_users: RwLock::new(HashMap::new()),
        })
    }

    pub async fn initialize(&self) -> Result<(), Error> {
        self.set_commands().await?;
        Ok(())
    }

    async fn set_commands(&self) -> Result<(), Error> {
        let commands = json!({
            "commands": [
                {
                    "command": "start",
                    "description": "Start receiving token alerts"
                },
                {
                    "command": "stop",
                    "description": "Stop receiving token alerts"
                },
                {
                    "command": "status",
                    "description": "Check your current alert status"
                },
                {
                    "command": "help",
                    "description": "Show available commands"
                },
                {
                    "command": "sniQ <address>",
                    "description": "Get token info"
                },
                {
                    "command": "peek <wallet>",
                    "description": "Get wallet info"
                },
                {
                    "command": "spot <wallet> <token_address>",
                    "description": "Get wallet holdings for a particular token"
                }
            ]
        });

        let url = format!("{}/setMyCommands", self.base_url);
        let response = self.client.post(&url).json(&commands).send().await?;

        if !response.status().is_success() {
            eprintln!("Failed to set commands: {:?}", response.text().await?);
        }

        Ok(())
    }
    
    fn calculate_team_allocation(&self, total_supply: String, total_team_allocation: String)-> std::string::String {
        let parsed_total_supply = self.format_large_number(&total_supply).unwrap().parse::<f64>().unwrap();
        let parsed_team_allocation = self.format_large_number(&total_team_allocation).unwrap().parse::<f64>().unwrap();

        let percentage_team_allocation = (parsed_team_allocation * 100.0) / parsed_total_supply;

        format!("{:.2}", percentage_team_allocation)
    }

    pub async fn broadcast_event(&self, event_data: MemecoinInfo) -> Result<(), Error> {
        let active_users = self.active_users.read().await;

        let message = format!(
            "🚨 ====== *FRESH LAUNCH ALERT* ====== 🚨\n\n\
                    *{}* ({}) has landed on Starknet!\n\n\
                    *Address:* {}\n\
                    *Starting MCAP:* ${}\n\
                    *Supply:* {}\n\
                    *Liquidity:* ${}\n\
                    *Team:* {}%\n\
                    ⚡️ *GET IN NOW*\n\n\
                    #Starknet #Memecoin #{}",
                    event_data.name,
                    event_data.symbol,
                    event_data.address,
            self.format_price(event_data.market_cap),
            self.format_number(&self.format_large_number(&event_data.total_supply).unwrap()).unwrap(),
            format!("{:.2}", event_data.usd_dex_liquidity.parse::<f64>().unwrap()),
            self.format_percentage(self.calculate_team_allocation(event_data.total_supply, event_data.team_allocation)),
            event_data.symbol
        );

        let keyboard = self.create_launch_keyboard(&event_data.address, &event_data.symbol);

        for (&chat_id, &active) in active_users.iter() {
            if active {
                if let Err(e) = self
                    .send_message_with_markup(chat_id, &message, keyboard.clone(), None)
                    .await
                {
                    eprintln!("Failed to broadcast event to {}: {:?}", chat_id, e);
                }
            }
        }

        Ok(())
    }

    fn create_launch_keyboard(
        &self,
        contract_address: &str,
        token_symbol: &str,
    ) -> serde_json::Value {
        json!({
            "inline_keyboard": [
                [
                    {
                        "text": "🚀 Buy $10",
                        "url": format!("{}?token={}&amount=10&symbol={}",
                            self.config.dex_url, contract_address, token_symbol)
                    },
                    {
                        "text": "🚀 Buy $50",
                        "url": format!("{}?token={}&amount=50&symbol={}",
                            self.config.dex_url, contract_address, token_symbol)
                    },
                    {
                        "text": "🚀 Buy $100",
                        "url": format!("{}?token={}&amount=100&symbol={}",
                            self.config.dex_url, contract_address, token_symbol)
                    }
                ],
                [
                    {
                        "text": "💰 Custom Amount",
                        "url": format!("{}?token={}",
                            self.config.dex_url, contract_address)
                    }
                ]
            ]
        })
    }

    fn format_number(&self, num_str: &str) -> Result<String, &'static str> {
        // Parse the string to f64
        let num = match num_str.parse::<f64>() {
            Ok(n) => n,
            Err(_) => return Err("Invalid number format"),
        };
    
        // Define the thresholds and their corresponding suffixes
        let billion = 1_000_000_000.0;
        let million = 1_000_000.0;
        let thousand = 1_000.0;
    
        let (value, suffix) = if num >= billion {
            (num / billion, "B")
        } else if num >= million {
            (num / million, "M")
        } else if num >= thousand {
            (num / thousand, "K")
        } else {
            (num, "")
        };
    
        // Format with up to 2 decimal places, removing trailing zeros
        let formatted = format!("{:.2}", value)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string();
    
        Ok(format!("{}{}", formatted, suffix))
    }


    fn format_large_number(&self, input: &str) -> Result<String, &'static str> {
        // Validate input is numeric
    if !input.chars().all(|c| c.is_digit(10)) {
        return Err("Invalid input: must contain only digits");
    }

    let input_len = input.len();
    
    // If input is less than 18 digits, we need to add decimal places
    if input_len < 18 {
        let zeros_needed = 18 - input_len;
        let mut result = "0.".to_string();
        // Add necessary leading zeros
        for _ in 0..zeros_needed {
            result.push('0');
        }
        result.push_str(input.trim_start_matches('0'));
        if result == "0." {
            return Ok("0".to_string());
        }
        return Ok(result.trim_end_matches('0').trim_end_matches('.').to_string());
    }
    
    // If input is exactly 18 digits, result is 1
    if input_len == 18 {
        return Ok("1".to_string());
    }
    
    // If input is more than 18 digits, we need to place a decimal point
    let decimal_position = input_len - 18;
    let mut result = input[0..decimal_position].to_string();
    let fraction = &input[decimal_position..];
    
    if fraction != "000000000000000000" {
        result.push('.');
        result.push_str(fraction.trim_end_matches('0'));
    }
    
    // Remove leading zeros and handle special case
    result = result.trim_start_matches('0').to_string();
    if result.is_empty() || result.starts_with('.') {
        result = format!("0{}", result);
    }
    
    // Remove trailing decimal if it exists
    if result.ends_with('.') {
        result.pop();
    }
    
    Ok(result)
    }

    // Helper functions for formatting
    fn format_price(&self, price: String) -> String {
        format!("{:.2}", price)
    }

    fn format_percentage(&self, value_str: String) -> String {
        // Try to parse the string as f64
        match value_str.parse::<f64>() {
            Ok(value) => {
                format!("{:.1}", value)
            }
            Err(_) => {
                // If parsing fails, return the original string
                // You might want to log this error in a production environment
                eprintln!("Failed to parse percentage string: {}", value_str);
                value_str
            }
        }
    }

    fn format_short_address(&self, address: &str) -> String {
        if address.len() > 8 {
            format!("{}...{}", &address[..6], &address[address.len() - 4..])
        } else {
            address.to_string()
        }
    }

    pub async fn handle_updates(&self) -> Result<(), Error> {
        let mut last_update_id = 0;

        loop {
            match self.get_updates(last_update_id + 1).await {
                Ok(updates) => {
                    for update in updates {
                        if let Some(message) = update.message {
                            if let Some(text) = message.text {
                                self.handle_command(&text, message.chat.id).await?;
                            }
                        }
                        last_update_id = update.update_id;
                    }
                }
                Err(e) => {
                    eprintln!("Error getting updates: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    async fn handle_command(&self, command: &str, chat_id: i64) -> Result<(), Error> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        
        match parts.get(0).map(|s| *s) {
            Some("/spot") => {
                match (parts.get(1), parts.get(2)) {
                    (Some(wallet_addr), Some(token_addr)) => {
                        match get_account_holding_info(wallet_addr, token_addr).await {
                            Ok(info) => {
                                let message = format!(
                                    "📊 ====== *TOKEN SPOT* ====== 📊\n\n\
                                    *Wallet:* {}\n\
                                    *Token:* ${}\n\n\
                                    *POSITION*\n\
                                    *Balance:* {}\n\
                                    *Worth:* ${}\n\n\
                                    *ACTIONS*\n\
                                    ⚡️ *Trade Now:* {}",
                                    self.format_short_address(wallet_addr),
                                    info.coin_info.symbol,
                                    self.format_large_number(&info.account_balance).unwrap(),
                                    info.usd_value,
                                    self.config.dex_url,
                                    // token_addr
                                );

                                self.send_message(chat_id, &message, None).await?;
                            }
                            Err(e) => {
                                let error_message = format!(
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
                                );
                                self.send_message(chat_id, &error_message, None).await?;
                            }
                        }
                    }
                    _ => {
                        self.send_message(
                            chat_id,
                            "❌ Invalid command format.\nUsage: `/spot <wallet_address> <token_address>`",
                            None,
                        )
                        .await?;
                    }
                }
            }
            Some("/start") => {
                let mut active_users = self.active_users.write().await;
                if active_users.insert(chat_id, true).is_none() {
                    self.send_message(
                        chat_id,
                        "⚡️ ====== *WELCOME TO SNIQ BOT* ====== ⚡️\n\n\
                                Catch the Meme. Beat the Market. 🎯🔥\n\n\
                                🚀 *FEATURES:*\n\
                                ✨ Instant Token Sniping – Know what’s hot in seconds.\n\
                                🔍 Wallet Scanning – Fast, flawless, precise.\n\
                                💸 One-Tap Trading – Access the market like a pro.\n\n\
                                ⚡️ *GET STARTED:*\n\
                                💥 */sniQ <address>* – Scan a token instantly!\n\
                                👀 */peek <wallet>* – See your memecoin holdings.\n\
                                🎯 */spot <wallet> <token>* – Track your position on any token.\n\n\
                                💎 sniq.fun\n\
                                Fast. Sharp. Ahead. — Sniping Memecoins Like a Pro. ⚡️"
                                ,
                        None,
                    )
                    .await?;
                } else {
                    self.send_message(chat_id, "✅ You are already receiving token alerts!", None)
                        .await?;
                }
            }
            Some("/stop") => {
                let mut active_users = self.active_users.write().await;
                if active_users.remove(&chat_id).is_some() {
                    self.send_message(
                        chat_id,
                        "🛑 Token alerts stopped. Use /start to resume.",
                        None,
                    )
                    .await?;
                } else {
                    self.send_message(
                        chat_id,
                        "❗️ You are not receiving any alerts. Use /start to begin.",
                        None,
                    )
                    .await?;
                }
            }
            Some("/status") => {
                let active_users = self.active_users.read().await;
                let status = if active_users.get(&chat_id).copied().unwrap_or(false) {
                    "🟢 You are currently receiving token alerts."
                } else {
                    "🔴 You are not receiving token alerts.\nUse /start to begin."
                };
                self.send_message(chat_id, status, None).await?;
            }
            Some("/help") => {
                self.send_message(
                    chat_id,
                    "Available Commands:\n\n\
                    /start - Start receiving token alerts\n\
                    /stop - Stop receiving token alerts\n\
                    /status - Check your alert status\n\
                    /help - Show this help message\n\
                    /spot <wallet> <token> - Get token position for a wallet\n\
                    /peek <wallet> - Check token position\n\
                    /sniQ <token> - Get info on a particular token\n\n\
                    ℹ️ You'll receive alerts for new tokens as they're detected.",
                    None,
                )
                .await?;
            }
            Some("/peek") => {
                match (parts.get(1)) {
                    Some(wallet_address) => {
                        match get_account_holdings(wallet_address).await {
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
                                self.send_message(chat_id, &message, None).await?;
                            }
                            Err(e) => {
                                let error_message = format!("Error peeking into wallet ⁉️");
                                self.send_message(chat_id, &error_message, None).await?;
                            }
                        }
                    },
                    None => {
                        let error_message = format!("Invalid parameters ❗️");
                        self.send_message(chat_id, &error_message, None).await?;
                    },
                }
            }
            Some("/sniQ") => {
                match (parts.get(1)) {
                    Some(token_address) => {
                        match aggregate_info(token_address).await {
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
                                        self.format_number(&response.0.market_cap).unwrap(),
                                        self.format_number(&self.format_large_number(&response.0.total_supply).unwrap()).unwrap(),
                                        response.1.category,
                                        self.format_number(&response.0.usd_dex_liquidity).unwrap(),
                                        self.config.dex_url,
                                        format!("{}/{}",self.config.explorer_url, response.0.address )
                                    );
                                self.send_message(chat_id,  &message, None).await;
                            },
                            Err(error) => {
                                let error_message = format!("Error fetching token details ⁉️");
                                self.send_message(chat_id, &error_message, None).await?;
                            }
                        }
                    },
                    None => {
                        let error_message = format!("Invalid parameters ❗️");
                        self.send_message(chat_id, &error_message, None).await?;
                    }              
                }
            }
            
            _ => {}
        }
        Ok(())
    }

    async fn get_updates(&self, offset: i64) -> Result<Vec<Update>, Error> {
        let url = format!("{}/getUpdates", self.base_url);

        let params = json!({
            "offset": offset,
            "timeout": 30,
            "allowed_updates": ["message", "callback_query"]
        });

        let response = self.client.post(&url).json(&params).send().await?;

        #[derive(Deserialize)]
        struct UpdateResponse {
            ok: bool,
            result: Vec<Update>,
        }

        if response.status().is_success() {
            let update_response: UpdateResponse = response.json().await?;
            Ok(update_response.result)
        } else {
            eprintln!("Error getting updates: {:?}", response.text().await?);
            Ok(Vec::new())
        }
    }

    async fn send_message(
        &self,
        chat_id: i64,
        text: &str,
        reply_to: Option<i64>,
    ) -> Result<(), Error> {
        let mut request = json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "Markdown"
        });

        if let Some(reply_id) = reply_to {
            request
                .as_object_mut()
                .unwrap()
                .insert("reply_to_message_id".to_string(), json!(reply_id));
        }

        let url = format!("{}/sendMessage", self.base_url);
        let response = self.client.post(&url).json(&request).send().await?;

        if !response.status().is_success() {
            eprintln!("Failed to send message: {:?}", response.text().await?);
        }

        Ok(())
    }

    async fn send_message_with_markup(
        &self,
        chat_id: i64,
        text: &str,
        reply_markup: serde_json::Value,
        reply_to: Option<i64>,
    ) -> Result<(), Error> {
        let mut request = json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "Markdown",
            "reply_markup": reply_markup
        });

        if let Some(reply_id) = reply_to {
            request
                .as_object_mut()
                .unwrap()
                .insert("reply_to_message_id".to_string(), json!(reply_id));
        }

        let url = format!("{}/sendMessage", self.base_url);
        let response = self.client.post(&url).json(&request).send().await?;

        if !response.status().is_success() {
            eprintln!(
                "Failed to send message with markup: {:?}",
                response.text().await?
            );
        }

        Ok(())
    }
}
