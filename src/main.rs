use std::sync::Arc;

use apibara_core::starknet::v1alpha2::{Event, FieldElement};
use dotenv::dotenv;
use kanshi::{config::Config, dna::IndexerService, utils::conversions::{apibara_field_as_felt, felt_as_apibara_field}};
use starknet::core::utils::get_selector_from_name;
use starknet_core::types::Felt;
use telegram::{TelegramBot, TelegramConfig};
use tokio::sync::mpsc;
use tokio::task;
use anyhow::{Context, Result};
use utils::{call::get_aggregate_call_data, event_parser::{CreationEvent, FromStarknetEventData, LaunchEvent}, market_cap::calculate_market_cap, types::ekubo::Memecoin};

mod constant;
mod utils;
mod telegram;

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

    // Initialize Telegram bot
    let tg_config = TelegramConfig::new();
    let tg_bot = match TelegramBot::new(tg_config) {
        Ok(bot) => {
            println!("Telegram bot initialized ✓");
            Arc::new(bot)
        }
        Err(e) => {
            eprintln!("Failed to initialize Telegram bot ❗️ {}", e);
            return;
        }
    };

    // Initialize the bot
    if let Err(e) = tg_bot.initialize().await {
        eprintln!("Failed to initialize Telegram bot commands ❗️ {}", e);
        return;
    }

    // Create Arc clones for different tasks
    let tg_bot_updates = Arc::clone(&tg_bot);
    let tg_bot_events = Arc::clone(&tg_bot);
    
    // Spawn Telegram bot handler in a separate task
    let telegram_handle = task::spawn(async move {
        if let Err(e) = tg_bot_updates.handle_updates().await {
            eprintln!("Error running Telegram bot ❗️ {}", e);
        }
    });

    
    // Spawn the indexer service in a separate task
    let indexer_handle = task::spawn(async move {
        if let Err(e) = service.await.run_forever_simplified(&tx).await {
            eprintln!("Error running Indexer ❗️ {:#}", e);
        }
    });

    // Spawn the event consumer in a separate task
    let consumer_handle = task::spawn(async move {
        while let Some(event) = rx.recv().await {
            if let Err(e) = process_event(event, &tg_bot_events).await {
                eprintln!("Error processing event ❗️ {}", e);
            }
        }
    });

    // Wait for both tasks to complete
    tokio::select! {
        _ = indexer_handle => println!("Indexer task completed"),
        _ = consumer_handle => println!("Consumer task completed"),
    }
}


async fn process_event(event: Event, tg_bot: &Arc<TelegramBot>) -> Result<()> {
    let event_selector = event.keys.first().context("No event selector")?;
    let event_data: Vec<Felt> = event.data.iter()
        .map(apibara_field_as_felt)
        .collect();
    match event_selector {
        selector if *selector == *CREATION_EVENT => {
            println!("New creation event: {:?}\n", event.from_address);
        }

        selector if *selector == *LAUNCH_EVENT => {
            let mut coin_data = Default::default();
            let decoded_data = decode_launch_data(event_data).await?;
            match get_aggregate_call_data(&decoded_data.memecoin_address.to_hex_string()).await {
                Ok(data) => {
                    coin_data = data.clone();
                    println!("{:?}", data)
                }
                Err(err) => {
                    println!("------- [Error] Aggregate Call -------");
                    println!("{:?}", err)
                },
            }

            match calculate_market_cap(coin_data.clone().total_supply, coin_data.clone().symbol).await {
                Ok(data) => {
                    println!("------- Coin Data -------");
                    println!("{:?}", data);

                    let event_data = Memecoin {
                        address: coin_data.address,
                        is_launched: coin_data.is_launched,
                        launch: coin_data.launch,
                        liquidity: coin_data.liquidity,
                        name: coin_data.name,
                        owner: coin_data.owner,
                        symbol: coin_data.symbol,
                        total_supply: coin_data.total_supply
                    };

                    if let Err(err) = tg_bot.broadcast_event(event_data, data).await {
                        println!("------- [Error] Telegram -------");
                        println!("{:?}", err)
                    }
                }
                Err(err) => {
                    println!("------- [Error] Market Cap -------");
                    println!("{:?}", err)
                },
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}

async fn decode_launch_data(event_data: Vec<Felt>) -> anyhow::Result<LaunchEvent, anyhow::Error> {
    let launch_event: LaunchEvent =
        LaunchEvent::from_starknet_event_data(event_data).context("Parsing Launch Event")?;
    Ok(launch_event)
}