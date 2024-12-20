use std::{env, fmt::format};
use serde::{Deserialize, Serialize};

use reqwest::{Client, Error};

#[derive(Debug, Serialize)]
struct SendMessageRequest {
    chat_id: i64,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to_message_id: Option<i64>,
}

pub struct TelegramBot {
    token: String,
    client: Client,
    base_url: String,
}

impl  TelegramBot {
    pub fn new() -> Result<Self, Error> {
        let token = env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN not found");
        let client = Client::new();
        let base_url = format!("https://api.telegram.org/bot{}", token);

        Ok(Self {
            token,
            client,
            base_url
        })
    }

    pub async fn send_message(&self, chat_id: i64, text: &str, reply_to: Option<i64>) -> Result<(), Error> {
        let url = format!("{}/sendMessage", self.base_url);

        let message = SendMessageRequest {
            chat_id,
            text: text.to_string(),
            reply_to_message_id: reply_to
        };

        let response = self.client.post(&url).body(&message).send().await?;
        if !response.status().is_success() {
            println!("Error sending message: {:?}", response.text().await?);
        }

        Ok(())
    }
}