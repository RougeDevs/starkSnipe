use serde::{Deserialize, Serialize};
use reqwest::{Client, Error};
#[derive(Debug, Serialize)]
struct SendMessageRequest {
    chat_id: i64,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_markup: Option<InlineKeyboardMarkup>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to_message_id: Option<i64>,
}

#[derive(Debug, Serialize)]
struct InlineKeyboardMarkup {
    inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

#[derive(Debug, Serialize)]
struct InlineKeyboardButton {
    text: String,
    kind: InlineKeyboardButtonKind,
}

#[derive(Debug, Serialize)]
enum InlineKeyboardButtonKind {
    Callback(String),
    Url(String),
    // Add other variants as needed
}

pub struct TelegramBot {
    token: String,
    client: Client,
    base_url: String,
}

impl  TelegramBot {
    pub fn new() -> Result<Self, Error> {
        let token = std::env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN not found");
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
            reply_markup: None,
            reply_to_message_id: reply_to
        };
        let json_body = serde_json::to_string(&message)
            .expect("Failed to serialize message");
        let response = self.client.post(&url).json(&json_body).send().await?;
        if !response.status().is_success() {
            println!("Error sending message: {:?}", response.text().await?);
        }

        Ok(())
    }

    pub async fn send_message_with_buttons(&self, chat_id: i64, text: &str, reply_to: Option<i64>) -> Result<(), Error> {
        let url = format!("{}/sendMessage", self.base_url);

        let keyboard = InlineKeyboardMarkup {
            inline_keyboard: vec![vec![
                InlineKeyboardButton {
                    text: "Buy".to_string(),
                    kind: InlineKeyboardButtonKind::Callback("action:buy".to_string())
                },
                InlineKeyboardButton {
                    text: "Sell".to_string(),
                    kind: InlineKeyboardButtonKind::Callback("action:sell".to_string()),
                }
            ]]
        };

        let message = SendMessageRequest {
            chat_id,
            text: text.to_string(),
            reply_markup: Some(keyboard),
            reply_to_message_id: reply_to
        };

        let json_body = serde_json::to_string(&message).expect("Failed to serialize message");
        let response = self.client.post(&url).json(&json_body).send().await?;
        if !response.status().is_success() {
            println!("Error sending message: {:?}", response.text().await?);
        }

        Ok(())
    }
}