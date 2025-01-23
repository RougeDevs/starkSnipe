use messages::{create_launch_keyboard, generate_broadcast_event, handle_peek_command, handle_sniq_command, handle_spot_command};
use reqwest::{Client, Error};
use serde::Deserialize;
use serde_json::json;
use types::common::Update;
use utils::{calculate_team_allocation, format_large_number, format_number, format_percentage, format_price, format_short_address, is_valid_starknet_address};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::utils::info_aggregator::{aggregate_info, get_account_holding_info, get_account_holdings};
use crate::utils::types::common::MemecoinInfo;

pub mod types;
pub mod utils;
pub mod messages;

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
        let commands = json!([
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
                "command": "sniq",
                "description": "Get token info by address"
            },
            {
                "command": "peek",
                "description": "Get wallet info by address"
            },
            {
                "command": "spot",
                "description": "Get wallet token holdings"
            }
        ]);

        let url = format!("{}/setMyCommands", self.base_url);
        let response = self.client.post(&url).json(&commands).send().await?;

        if !response.status().is_success() {
            eprintln!("Failed to set commands: {:?}", response.text().await?);
        }

        Ok(())
    }

    pub async fn broadcast_event(&self, event_data: MemecoinInfo) -> Result<(), Error> {
        let active_users = self.active_users.read().await;

        let message = generate_broadcast_event(event_data.clone());

        let keyboard = create_launch_keyboard(&self.config.dex_url,&event_data.address, &event_data.symbol);
        // println!("active_users -> {}", active_users.clone().len());
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
        println!("handle command invoked");
        match parts.get(0).map(|s| *s) {
            Some("/spot") => {
                match (parts.get(1), parts.get(2)) {
                    (Some(wallet_address), Some(token_address)) => {
                            let message = handle_spot_command(wallet_address.to_string(), token_address.to_string(), &self.config.dex_url).await;
                            self.send_message(chat_id, &message, None).await?;
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
                println!("received start info -> {:?}", active_users.clone());
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
                        let message = handle_peek_command(wallet_address.to_string()).await;
                        self.send_message(chat_id, &message, None).await;
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
                        let message = handle_sniq_command(token_address.to_string(), &self.config.dex_url, &self.config.explorer_url).await;
                        self.send_message(chat_id, &message, None).await?;
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
        println!("tg bot response -> {:?}", response);

        if !response.status().is_success() {
            eprintln!(
                "Failed to send message with markup: {:?}",
                response.text().await?
            );
        }

        Ok(())
    }
}
