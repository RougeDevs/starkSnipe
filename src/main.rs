use apibara_core::starknet::v1alpha2::{Event, FieldElement};
use dotenv::dotenv;
use kanshi::{config::Config, dna::IndexerService, utils::conversions::{apibara_field_as_felt, felt_as_apibara_field}};
use starknet::core::utils::get_selector_from_name;
use starknet_core::types::Felt;
use tokio::sync::mpsc;
use tokio::task;
use anyhow::{Context, Result};
use utils::{call::get_aggregate_call_data, event_parser::{CreationEvent, FromStarknetEventData, LaunchEvent}, market_cap::calculate_market_cap};

mod constant;
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


#[tokio::main]
async fn main() {
    dotenv().ok();

    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
    
    // Load configurations
    let config = match Config::new() {
        Ok(config) => {
            println!("Configurations loaded ✓");
            config
        }
        Err(e) => {
            eprintln!("Failed to load configuration ❗️ {}", e);
            return;
        }
    };

    // Create the IndexerService instance
    let service = IndexerService::new(config);
    
    // Spawn the indexer service in a separate task
    let indexer_handle = task::spawn(async move {
        if let Err(e) = service.await.run_forever_simplified(&tx).await {
            eprintln!("Error running Indexer ❗️ {:#}", e);
        }
    });

    // Spawn the event consumer in a separate task
    let consumer_handle = task::spawn(async move {
        while let Some(event) = rx.recv().await {
            println!("🔥 Received Event: {:?}\n\n", event);
            // Add your event processing logic here
            // For example:
            process_event(event).await;
        }
    });

    // Wait for both tasks to complete
    tokio::select! {
        _ = indexer_handle => println!("Indexer task completed"),
        _ = consumer_handle => println!("Consumer task completed"),
    }
}


async fn process_event(event: Event) -> Result<()> {
    let event_selector = event.keys.first().context("No event selector")?;
    let event_data: Vec<Felt> = event.data.iter().map(apibara_field_as_felt).collect();
    match event_selector {
        selector if *selector == *CREATION_EVENT => {
            println!("New creation event: {:?}\n", event.from_address);
        }

        selector if *selector == *LAUNCH_EVENT => {
            println!("Got Launch Event: {:?}", event.from_address);
            let mut coin_data = Default::default();
            let decoded_data = decode_launch_data(event_data).await?;
            match get_aggregate_call_data(&decoded_data.memecoin_address.to_string()).await {
                Ok(data) => {
                    coin_data = data.clone();
                    println!("{:?}", data)
                }
                Err(err) => eprintln!("Error: {:?}", err),
            }

            match calculate_market_cap(coin_data.total_supply, coin_data.symbol).await {
                Ok(data) => {
                    println!("------- Coin Data -------");
                    println!("{:?}", data)
                }
                Err(err) => eprintln!("Error: {:?}", err),
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}

async fn decode_launch_data(event_data: Vec<Felt>) -> anyhow::Result<LaunchEvent, anyhow::Error> {
    let launch_event: LaunchEvent =
        LaunchEvent::from_starknet_event_data(event_data).context("Parsing Launch Event")?;
    println!("{:?}", launch_event);
    Ok(launch_event)
}