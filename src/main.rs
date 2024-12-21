use std::time::Duration;
use dotenv::dotenv;

use provider::{Monitor, StarknetProviderError, StarknetProviderOptions};
use reqwest::{header::{HeaderMap, HeaderValue}, Error as ReqwestError};
use starknet::core::types::{BlockId, EventFilter, Felt};
use telegram::TelegramBot;
use tokio::runtime::Builder;
use url::Url;

mod provider;
mod telegram;

#[derive(Debug)]
enum AppError {
    Provider(StarknetProviderError),
    Reqwest(ReqwestError),
    Telegram(String),
    Url(url::ParseError),
    Other(String),
}

// Implement conversions from specific errors to AppError
impl From<StarknetProviderError> for AppError {
    fn from(err: StarknetProviderError) -> Self {
        AppError::Provider(err)
    }
}

impl From<ReqwestError> for AppError {
    fn from(err: ReqwestError) -> Self {
        AppError::Reqwest(err)
    }
}


async fn run_monitor() -> Result<(), AppError> {
    let url = Url::parse("https://starknet-mainnet.infura.io/v3/edd0fd50d7d948d58c513f38e5622da2").unwrap();
    let mut headers = HeaderMap::new();
    let hex_address = "0x04718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d";
    let address = Felt::from_hex(hex_address).unwrap();
    
    headers.insert(
        "Authorization", 
        HeaderValue::from_static("edd0fd50d7d948d58c513f38e5622da2")
    );
    
    let options = StarknetProviderOptions {
        timeout: Duration::from_secs(30),
        headers
    };
    
    let listener = Monitor::new(url, options).expect("Failed to start monitor");
    // let transfer_selector = get_selector_from_name("Transfer").unwrap();
    
    let filter = EventFilter {
        from_block: Some(BlockId::Number(1100)),
        to_block: None,
        address: Some(address),
        keys: Some(vec![]),
    };

    listener.listen_for_events(filter, |events| {
        for event in events.events {
            println!("New event received:");
            println!("  From address: {:?}", event.from_address);
            println!("  Block number: {:?}", event.block_number);
            println!("  Transaction hash: {:?}", event.transaction_hash);
            println!("  Data: {:?}", event.data);
        }
    }).await?;

    Ok(())
}

fn main() -> Result<(),AppError> {
    dotenv().ok();
    let rt = Builder::new_multi_thread().enable_all().build().unwrap();
    // rt.block_on(run_monitor())?;

    // Trigger Telegram bot
    rt.block_on(async {
        let bot = TelegramBot::new().map_err(|e| AppError::Telegram(format!("Failed to initialise telegram bot: {}", e))).unwrap();
        let text = "This is a new chat test";
        bot.send_message_with_buttons(7257416467, text, None).await?;
        Ok(())
    })
}