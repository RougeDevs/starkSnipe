use std::sync::Arc;

use anyhow::{Context, Result};
use apibara_core::starknet::v1alpha2::{Event, FieldElement};
use axum::{routing::get, Router};
use dotenv::dotenv;
use kanshi::{
    config::Config,
    dna::IndexerService,
    utils::conversions::{apibara_field_as_felt, felt_as_apibara_field},
};
use shuttle_axum::ShuttleAxum;
use shuttle_runtime::SecretStore;
use starknet::core::utils::get_selector_from_name;
use starknet_core::types::Felt;
use telegram::{TelegramBot, TelegramConfig};
use tokio::sync::mpsc;
use tokio::task;
use utils::{
    event_parser::{CreationEvent, FromStarknetEventData, LaunchEvent},
    info_aggregator::aggregate_info,
};

mod constant;
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

#[shuttle_runtime::main]
pub async fn axum(
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> ShuttleAxum {
    dotenv().ok();
    println!("Welcome to sniQ");
   // Helper function to set environment variables safely
    let set_env_var = |key: &str| {
        if let Some(value) = secrets.get(key) {
            std::env::set_var(key, value);
        } else {
            eprintln!("Warning: {} not found in secrets", key);
        }
    };

    // Set all environment variables
    set_env_var("TELEGRAM_TOKEN");
    set_env_var("APIBARA_KEY");
    set_env_var("CONTRACT_ADDRESS");
    set_env_var("STARTING_BLOCK");
    set_env_var("EXPLORER_API");
    set_env_var("EXPLORER");
    set_env_var("EKUBO_CORE_ADDRESS");
    set_env_var("DEX_URL");
    set_env_var("EXPLORER_URL");
    set_env_var("WRITE_PATH");
    // Initialize a channel for event processing
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    // Load configurations from environment or config files
    let config = Config::new().context("Failed to load configuration")?;

    // Create the IndexerService instance
    let service = IndexerService::new(config);

    // Initialize Telegram bot
    let tg_config = TelegramConfig::new();
    let tg_bot = TelegramBot::new(tg_config).context("Failed to initialize Telegram bot")?;
    let tg_bot = Arc::new(tg_bot);

    // Initialize the bot
    tg_bot.initialize().await.context("Failed to initialize Telegram bot commands")?;


    // Clone Telegram bot for separate tasks
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

    // Return a basic Axum server with no routes
    let router = Router::new().route("/health", get(|| async { "OK" }));

    Ok(router.into())
}

async fn process_event(event: Event, tg_bot: &Arc<TelegramBot>) -> Result<()> {
    let event_selector = event.keys.first().context("No event selector")?;
    let event_data: Vec<Felt> = event.data.iter().map(apibara_field_as_felt).collect();
    match event_selector {
        selector if *selector == *CREATION_EVENT => {
            println!("New creation event: {:?}\n", event.from_address);
        }

        selector if *selector == *LAUNCH_EVENT => {
            let decoded_data = decode_launch_data(event_data).await?;
            match aggregate_info(&decoded_data.memecoin_address.to_hex_string()).await {
                Ok(data) => {
                    println!("{:?}", data.0);
                    if let Err(err) = tg_bot.broadcast_event(data.0).await {
                        println!("------- [Error] Telegram -------");
                        println!("{:?}", err)
                    }
                }
                Err(err) => {
                    println!("------- [Error] Aggregate Call -------");
                    println!("{:?}", err)
                }
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
