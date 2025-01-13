use std::{sync::Arc, time::Duration};

use anyhow::{ Result};
use dotenv::dotenv;

use kanshi::{config::Config, dna::{EventData, IndexerService}, utils::conversions::{field_to_hex_string, field_to_string}};
use provider::{Monitor, StarknetProviderError, StarknetProviderOptions};
use reqwest::{header::{HeaderMap, HeaderValue}, Error as ReqwestError};
use starknet::core::types::{BlockId, EventFilter, Felt};
use telegram::TelegramBot;
use apibara_core::starknet::v1alpha2::{Event, FieldElement};
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


async fn run_monitor() -> Result<(), anyhow::Error> {
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
    }).await;

    Ok(())
}

fn main() -> Result<(),anyhow::Error> {
    dotenv().ok();

    
    let rt = Builder::new_multi_thread().enable_all().build().unwrap();
    // rt.block_on(run_monitor())?;

    // Trigger Telegram bot
    // rt.block_on(async {
    //     let bot = TelegramBot::new().map_err(|e| AppError::Telegram(format!("Failed to initialise telegram bot: {}", e))).unwrap();
    //     let text = "This is a new chat test";
    //     bot.send_message_with_buttons(7257416467, text, None).await?;
    //     Ok(())
    // });

    rt.block_on(async {
        let config = Config::new().expect("Failed to load configuration");
        // Create the IndexerService instance
        let mut indexer_service = IndexerService::new(config).await;
        

        // Define a handler function for processing events
        let handler = |block_number: u64, event: &Event| {
            // Process the event asynchronously
            let event_arc = Arc::new(event.clone());
            tokio::spawn(async move {
                match handle_event(block_number,&*event_arc).await {
                    Ok(Some(event_data)) => {
                        println!(
                            "Event processed successfully at block {}",
                            event_data.block_number
                        );
                    }
                    Ok(None) => println!("Event data incomplete, skipping."),
                    Err(err) => eprintln!("Error processing event: {:?}", err),
                }
            });
        };
        indexer_service = indexer_service.with_handler(handler);
        // Run the indexer service with our handler
        if let Err(err) = indexer_service.run_forever().await {
            eprintln!("Error while running the indexer service: {:?}", err);
        }

        Ok(())
    })

}

async fn handle_event(block_number: u64, event: &apibara_core::starknet::v1alpha2::Event) -> Result<Option<EventData>> {
    let from_address = match &event.from_address {
        Some(field) => field_to_hex_string(field),
        None => return Err(anyhow::anyhow!("Missing from_address in event")),
    };

    // Parse event data
    if event.data.len() >= 5 {
        let owner = field_to_hex_string(&event.data[0]);
        let name = field_to_string(&event.data[1]);
        let symbol = field_to_string(&event.data[2]);
        
        let supply_low = field_to_hex_string(&event.data[3]);
        let supply_high = field_to_hex_string(&event.data[4]);
        
        // Convert hex strings to numbers, removing '0x' prefix
        let low_value = u128::from_str_radix(supply_low.trim_start_matches("0x"), 16)
            .unwrap_or(0);
        let high_value = u128::from_str_radix(supply_high.trim_start_matches("0x"), 16)
            .unwrap_or(0);

        // Format the number with proper decimal places (assuming 18 decimals for the token)
        let initial_supply = if high_value == 0 {
            low_value.to_string()
        } else {
            // If high part exists, combine them
            format!("{}{:016x}", high_value, low_value)
        };
        
        let memecoin_address = if event.data.len() > 5 {
            field_to_hex_string(&event.data[5])
        } else {
            "Not provided".to_string()
        };

        // Create EventData struct
        let event_data = EventData {
            block_number,
            from_address,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            transaction_hash: "0x0".to_string(), // placeholder
            data: vec![
                owner.clone(),
                name.clone(),
                symbol.clone(),
                initial_supply.clone(),
                memecoin_address.clone(),
            ],
        };


        // Print formatted event information
        println!("\nNew Memecoin Launch Event:");
        println!("------------------------");
        println!("Block Number: {}", block_number);
        println!("Contract Address: {}",event_data.from_address);
        println!("Owner Address: {}", owner);
        println!("Name: {}", name);
        println!("Symbol: {}", symbol);
        println!("Initial Supply: {}", initial_supply);
        println!("Memecoin Address: {}", memecoin_address);
        println!("------------------------\n");

        tokio::spawn(async move {
            let bot = TelegramBot::new().map_err(|e| AppError::Telegram(format!("Failed to initialise telegram bot: {}", e))).unwrap();
            let address = memecoin_address.clone();
            let name = name.clone();
            let symbol = symbol.clone();
            let text = format!(
                "New Memecoin Launched:\n\nToken Address: {}\nName: {}\nSymbol: {}",
                address, name, symbol
            );
            bot.send_message_with_buttons(6722922954, &text.clone(), None).await?;
            Ok::<(), AppError>(())
        });

        Ok(Some(event_data))
    } else {
        println!("Warning: Event data doesn't contain expected number of fields");
        println!("Received data: {:?}", event.data);
        Ok(None)
    }
}
