use std::sync::Arc;

use anyhow::{ Context, Result};
use dotenv::dotenv;

use kanshi::{config::Config, dna::IndexerService, utils::conversions::{apibara_field_as_felt, felt_as_apibara_field}};
use provider::StarknetProviderError;
use reqwest::Error as ReqwestError;
use starknet::core::types::Felt;
use starknet::core::utils::get_selector_from_name;
use telegram::TelegramBot;
use apibara_core::starknet::v1alpha2::{Event, FieldElement};
use tokio::runtime::Builder;
use url::Url;
use utils::event_parser::{CreationEvent, LaunchEvent, FromStarknetEventData};

mod provider;
mod telegram;
mod utils;

lazy_static::lazy_static! {
    pub static ref CREATION_EVENT: FieldElement = felt_as_apibara_field(&get_selector_from_name("MemecoinCreated").unwrap());
    pub static ref LAUNCH_EVENT: FieldElement = felt_as_apibara_field(&get_selector_from_name("MemecoinLaunched").unwrap());
}
#[derive(Debug)]
enum EventType {
    Creation(CreationEvent),
    Launch(LaunchEvent),
}


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

fn main() -> Result<(),anyhow::Error> {
    dotenv().ok();

    
    let rt = Builder::new_multi_thread().enable_all().build().unwrap();

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
                    Ok(()) => {
                    }
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

async fn handle_event(block_number: u64, event: &Event) -> Result<()> {
    let event_selector = event.keys.first().context("No event selector")?;
    let event_data: Vec<Felt> = event.data.iter().map(apibara_field_as_felt).collect();
    let event: EventType;
    match event_selector {
        selector if selector == &*CREATION_EVENT => {
            eprintln!("Got Creation Event at block: {:?}", block_number);
            event = EventType::Creation(decode_creation_data(event_data).await?);
        }
        selector if selector == &*LAUNCH_EVENT => {
            eprintln!("Got Launch Event at block: {:?}", block_number);
            event = EventType::Launch(decode_launch_data(event_data).await?);
        }
        _ => unreachable!(),
    }
    match event {
        EventType::Creation(_) => {
            // Handle CreationEvent
            println!("{:?}", event);
        }
        EventType::Launch(_) => {
            // Handle LaunchEvent
            println!("{:?}", event);
        }
    }
    Ok(())
}

async fn decode_creation_data(event_data: Vec<Felt>) -> anyhow::Result<CreationEvent, anyhow::Error>{
    let creation_event: CreationEvent = CreationEvent::from_starknet_event_data(event_data).context("Parsing Creation Event")?;
    Ok(creation_event)
    
}
async fn decode_launch_data(event_data: Vec<Felt>) -> anyhow::Result<LaunchEvent, anyhow::Error>{
    let launch_event: LaunchEvent = LaunchEvent::from_starknet_event_data(event_data).context("Parsing Launch Event")?;
    Ok(launch_event)
}
