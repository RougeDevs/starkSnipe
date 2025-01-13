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
    
        // Format the inline keyboard properly according to Telegram's API requirements
        let keyboard = serde_json::json!({
            "inline_keyboard": [
                [
                    {
                        "text": "Buy",
                        "callback_data": "action:buy"
                    },
                    {
                        "text": "Sell",
                        "callback_data": "action:sell"
                    }
                ]
            ]
        });

        // Create the message request with the properly formatted keyboard
        let message = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "reply_markup": keyboard,
            "reply_to_message_id": reply_to
        });
    
        let response = self.client
            .post(&url)
            .json(&message)  // Send the JSON directly
            .send()
            .await?;
    
        if !response.status().is_success() {
            let error_text = response.text().await?;
            eprintln!("Error sending message: {}", error_text);
        }
    
        Ok(())
    }
}