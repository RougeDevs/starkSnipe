use kanshi::dna::EventData;
use serde::Deserialize;
use reqwest::{Client, Error};
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::Duration;
use serde_json::json;

use crate::utils::event_parser::CreationEvent;
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
            token: std::env::var("TELEGRAM_TOKEN")
                .expect("TELEGRAM_TOKEN not found"),
            dex_url: std::env::var("DEX_URL")
                .unwrap_or_else(|_| "https://app.avnu.fi".to_string()),
            explorer_url: std::env::var("EXPLORER_URL")
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
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        
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
                }
            ]
        });

        let url = format!("{}/setMyCommands", self.base_url);
        let response = self.client
            .post(&url)
            .json(&commands)
            .send()
            .await?;

        if !response.status().is_success() {
            eprintln!("Failed to set commands: {:?}", response.text().await?);
        }

        Ok(())
    }

    pub async fn broadcast_event(&self, event_data: EventType) -> Result<(), Error> {
        let active_users = self.active_users.read().await;
        
        let mut message = "";

        match event_data {
            EventType::Creation(creation_event) => {
                message = &format!(
                    "🚨 *New Token Event Detected*\n\n\
                    *Token Details*\n\
                    Owner: `{}`\n\
                    Name: `{}`\n\
                    Symbol: `{}`\n\
                    Supply: `{}`\n\
                    Contract: `{}`\n\n\
                    Block Number: `{}`",
                    // event_data.data[0], // owner
                    // event_data.data[1], // name
                    // event_data.data[2], // symbol
                    // event_data.data[3], // supply
                    // event_data.data[4], // contract address
                    // event_data.block_number
                );
            },
            EventType::Launch(launch_event) => todo!(),
        }
        let keyboard = self.create_event_keyboard("0x0"); // Using contract address

        for (&chat_id, &active) in active_users.iter() {
            if active {
                if let Err(e) = self.send_message_with_markup(
                    chat_id,
                    &message,
                    keyboard.clone(),
                    None
                ).await {
                    eprintln!("Failed to broadcast event to {}: {:?}", chat_id, e);
                }
            }
        }

        Ok(())
    }

    fn create_event_keyboard(&self, contract_address: &str) -> serde_json::Value {
        json!({
            "inline_keyboard": [
                [
                    {
                        "text": "🔍 View on Explorer",
                        "url": format!("{}/contract/{}", self.config.explorer_url, contract_address)
                    }
                ],
                [
                    {
                        "text": "💱 Trade on Avnu",
                        "url": format!("{}/swap?inputToken=ETH&outputToken={}", 
                            self.config.dex_url, contract_address)
                    }
                ]
            ]
        })
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
        match command {
            "/start" => {
                let mut active_users = self.active_users.write().await;
                if active_users.insert(chat_id, true).is_none() {
                    self.send_message(
                        chat_id,
                        "🚀 Welcome! You will now receive token alerts.\n\n\
                        Use /help to see available commands.",
                        None
                    ).await?;
                } else {
                    self.send_message(
                        chat_id,
                        "✅ You are already receiving token alerts!",
                        None
                    ).await?;
                }
            },
            "/stop" => {
                let mut active_users = self.active_users.write().await;
                if active_users.remove(&chat_id).is_some() {
                    self.send_message(
                        chat_id,
                        "🛑 Token alerts stopped. Use /start to resume.",
                        None
                    ).await?;
                } else {
                    self.send_message(
                        chat_id,
                        "❗️ You are not receiving any alerts. Use /start to begin.",
                        None
                    ).await?;
                }
            },
            "/status" => {
                let active_users = self.active_users.read().await;
                let status = if active_users.get(&chat_id).copied().unwrap_or(false) {
                    "🟢 You are currently receiving token alerts."
                } else {
                    "🔴 You are not receiving token alerts.\nUse /start to begin."
                };
                self.send_message(chat_id, status, None).await?;
            },
            "/help" => {
                self.send_message(
                    chat_id,
                    "Available Commands:\n\n\
                    /start - Start receiving token alerts\n\
                    /stop - Stop receiving token alerts\n\
                    /status - Check your alert status\n\
                    /help - Show this help message\n\n\
                    ℹ️ You'll receive alerts for new tokens as they're detected.",
                    None
                ).await?;
            },
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

        let response = self.client
            .post(&url)
            .json(&params)
            .send()
            .await?;

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
        reply_to: Option<i64>
    ) -> Result<(), Error> {
        let mut request = json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "Markdown"
        });

        if let Some(reply_id) = reply_to {
            request.as_object_mut().unwrap().insert(
                "reply_to_message_id".to_string(),
                json!(reply_id)
            );
        }

        let url = format!("{}/sendMessage", self.base_url);
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;

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
        reply_to: Option<i64>
    ) -> Result<(), Error> {
        let mut request = json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "Markdown",
            "reply_markup": reply_markup
        });

        if let Some(reply_id) = reply_to {
            request.as_object_mut().unwrap().insert(
                "reply_to_message_id".to_string(),
                json!(reply_id)
            );
        }

        let url = format!("{}/sendMessage", self.base_url);
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            eprintln!("Failed to send message with markup: {:?}", response.text().await?);
        }

        Ok(())
    }
}