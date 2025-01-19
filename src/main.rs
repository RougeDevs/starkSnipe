use std::{
    panic,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
};

use anyhow::{Context, Result};
use backtrace::Backtrace;
use dotenv::dotenv;

use kanshi::{
    config::Config,
    dna::IndexerService,
    utils::conversions::{apibara_field_as_felt, felt_as_apibara_field},
};
use reqwest::Error as ReqwestError;
use starknet::core::utils::get_selector_from_name;
use starknet::{core::types::Felt, macros::selector};
// use telegram::{TelegramBot, TelegramConfig};
use apibara_core::starknet::v1alpha2::{Event, FieldElement};
use constant::constants::{selector_to_str, Selector};
use tokio::{runtime::Builder, sync::Semaphore};
use tracing::{error, info};
use utils::{
    call::get_aggregate_call_data,
    event_parser::{CreationEvent, FromStarknetEventData, LaunchEvent},
    market_cap::calculate_market_cap,
};

// mod telegram;
mod constant;
mod utils;

lazy_static::lazy_static! {
    pub static ref CREATION_EVENT: FieldElement = felt_as_apibara_field(&get_selector_from_name("MemecoinCreated").unwrap());
    pub static ref LAUNCH_EVENT: FieldElement = felt_as_apibara_field(&get_selector_from_name("MemecoinLaunched").unwrap());
    static ref CONCURRENT_CALLS: Semaphore = Semaphore::new(5); // Limit concurrent calls
    static ref LAST_PROCESSED_BLOCK: AtomicU64 = AtomicU64::new(0);
    static ref PANIC_COUNT: AtomicUsize = AtomicUsize::new(0);
}

#[derive(Debug)]
enum EventType {
    Creation(CreationEvent),
    Launch(LaunchEvent),
}

#[derive(Debug)]
enum AppError {
    Reqwest(ReqwestError),
    Telegram(String),
    Url(url::ParseError),
    Other(String),
}

impl From<ReqwestError> for AppError {
    fn from(err: ReqwestError) -> Self {
        AppError::Reqwest(err)
    }
}

fn setup_panic_handler() {
    panic::set_hook(Box::new(|panic_info| {
        PANIC_COUNT.fetch_add(1, Ordering::SeqCst);

        let backtrace = Backtrace::new();
        error!(
            "Thread panic occurred: {:?}\nLocation: {:?}\nBacktrace: {:?}",
            panic_info
                .payload()
                .downcast_ref::<&str>()
                .unwrap_or(&"<panic message not available>"),
            panic_info.location(),
            backtrace
        );
    }));
}

fn main() -> Result<(), anyhow::Error> {
    dotenv().ok();
    setup_panic_handler();
    let rt = Builder::new_multi_thread().enable_all().build().unwrap();
    let memecoin_address = "0x03b405a98c9e795d427fe82cdeeeed803f221b52471e3a757574a2b4180793ee";

    rt.block_on(async {
        let mut res1 = Default::default();
        match get_aggregate_call_data(&memecoin_address).await {
            Ok(data) => {
                res1 = data.clone();
                println!("{:?}", data)
            }
            Err(err) => eprintln!("Error: {:?}", err),
        }

        match calculate_market_cap(res1.total_supply, res1.symbol).await {
            Ok(data) => {
                println!("{:?}", data)
            }
            Err(err) => eprintln!("Error: {:?}", err),
        }

        // let config = Config::new().expect("Failed to load configuration");
        // let mut indexer_service = IndexerService::new(config).await;

        // Define the event handler using `handle_event_async`
        // let handler = move |block_number: u64, event: &Event| {
        //     let event_clone = event.clone();
        //     // Use tokio::spawn to asynchronously handle events
        //     tokio::spawn(handle_event_async(block_number, event_clone));
        // };

        // // Initialize the IndexerService with the handler
        // indexer_service = indexer_service.with_handler(handler);

        // Run the indexer service
        // if let Err(err) = indexer_service.run_forever().await {
        //     eprintln!("Error while running the indexer service: {:?}", err);
        // }

        Ok(())
    })
}

async fn handle_event_async(block_number: u64, event: Event) {
    // Use .await here to ensure completion
    match handle_event(block_number, &event).await {
        Ok(_) => {
            info!("Successfully processed event at block {}", block_number);
        }
        Err(err) => {
            error!(
                "Error processing event at block {}: {:?}",
                block_number, err
            );
        }
    }
}

async fn handle_event(block_number: u64, event: &Event) -> Result<()> {
    let event_selector = event.keys.first().context("No event selector")?;
    let event_data: Vec<Felt> = event.data.iter().map(apibara_field_as_felt).collect();

    match event_selector {
        // selector if selector == &*CREATION_EVENT => {
        //     eprintln!("Got Creation Event at block: {:?}", block_number);
        //     let creation_event = decode_creation_data(event_data).await?;
        //     println!("Creation Event: {:?}", creation_event);
        // }
        selector if selector == &*LAUNCH_EVENT => {
            eprintln!("Got Launch Event at block: {:?}", block_number);
            let lock = tokio::sync::Mutex::new(());

            let result = {
                let _lock = lock.lock().await;
                let decoded_data = decode_launch_data(event_data).await?;
                process_launch_event(block_number, decoded_data).await
            };
            // let decoded_data = decode_launch_data(event_data).await?;

            // Process launch event with rate limiting and error handling
            // if let Err(error) = process_launch_event(block_number, decoded_data).await {
            //     println!("Processing error: {:?}", error);
            // } else {
            //     println!("Procssed");
            // }
        }
        _ => unreachable!(),
    }

    Ok(())
}

async fn process_launch_event(block_number: u64, launch_event: LaunchEvent) -> Result<()> {
    let last_processed = LAST_PROCESSED_BLOCK.load(Ordering::Relaxed);
    if block_number <= last_processed {
        info!("Skipping already processed block {}", block_number);
        return Ok(());
    }

    let _permit = CONCURRENT_CALLS.acquire().await?;
    let memecoin_address = launch_event.memecoin_address.to_string();

    info!("Processing launch event for memecoin: {}", memecoin_address);

    match get_aggregate_call_data(&memecoin_address).await {
        Ok(response) => {
            info!("Successfully processed memecoin at block {}", block_number);
            info!("Parsed memecoin data {}", response);
            LAST_PROCESSED_BLOCK.store(block_number, Ordering::Relaxed);
            Ok(())
        }
        Err(error) => {
            error!(
                "Failed to process memecoin at block {}: {:?}",
                block_number, error
            );
            // Spawn retry task in the background
            tokio::spawn(async move {
                retry_failed_call(&memecoin_address, block_number).await;
            });
            Err(error.into())
        }
    }
}

async fn retry_failed_call(address: &str, block_number: u64) {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_MS: u64 = 1000;

    for retry in 0..MAX_RETRIES {
        tokio::time::sleep(tokio::time::Duration::from_millis(
            RETRY_DELAY_MS * (retry as u64 + 1),
        ))
        .await;

        match get_aggregate_call_data(address).await {
            Ok(result) => {
                println!(
                    "Retry succeeded for memecoin {} on attempt {}: {:?}",
                    address,
                    retry + 1,
                    result
                );
                LAST_PROCESSED_BLOCK.store(block_number, Ordering::Relaxed);
                return;
            }
            Err(e) => {
                eprintln!(
                    "Retry {} failed for memecoin {}: {:?}",
                    retry + 1,
                    address,
                    e
                );
            }
        }
    }

    eprintln!("All retries failed for memecoin {}", address);
}

async fn decode_creation_data(
    event_data: Vec<Felt>,
) -> anyhow::Result<CreationEvent, anyhow::Error> {
    let creation_event: CreationEvent =
        CreationEvent::from_starknet_event_data(event_data).context("Parsing Creation Event")?;
    Ok(creation_event)
}
async fn decode_launch_data(event_data: Vec<Felt>) -> anyhow::Result<LaunchEvent, anyhow::Error> {
    let launch_event: LaunchEvent =
        LaunchEvent::from_starknet_event_data(event_data).context("Parsing Launch Event")?;
    println!("{:?}", launch_event);
    Ok(launch_event)
}
