use std::{
    future::Future,
    panic,
    pin::Pin,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
};

use anyhow::{Context, Result};
use backtrace::Backtrace;
use dotenv::dotenv;
use kanshi::{config::Config, dna::IndexerService, utils::conversions::{apibara_field_as_felt, felt_as_apibara_field}};
use reqwest::Error as ReqwestError;
use starknet::{core::{types::Felt, utils::get_selector_from_name}, providers::{jsonrpc::HttpTransport, JsonRpcClient}};
use apibara_core::starknet::v1alpha2::{Event, FieldElement};
use tokio::{
    sync::{mpsc, Semaphore, Mutex},
    task,
};
use url::Url;
use utils::{call::{get_aggregate_call_data, AggregateError}, event_parser::{CreationEvent, FromStarknetEventData, LaunchEvent}};
use constant::constants::{selector_to_str, Selector};
use tracing::{error, info};

mod utils;
mod constant;

lazy_static::lazy_static! {
    pub static ref CREATION_EVENT: FieldElement = felt_as_apibara_field(&get_selector_from_name("MemecoinCreated").unwrap());
    pub static ref LAUNCH_EVENT: FieldElement = felt_as_apibara_field(&get_selector_from_name("MemecoinLaunched").unwrap());
    static ref CONCURRENT_CALLS: Semaphore = Semaphore::new(5);
    static ref LAST_PROCESSED_BLOCK: AtomicU64 = AtomicU64::new(0);
    static ref PANIC_COUNT: AtomicUsize = AtomicUsize::new(0);
}

type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug)]
enum EventType {
    Creation(CreationEvent),
    Launch(LaunchEvent),
}

fn setup_panic_handler() {
    panic::set_hook(Box::new(|panic_info| {
        PANIC_COUNT.fetch_add(1, Ordering::SeqCst);
        let backtrace = Backtrace::new();
        error!(
            "Thread panic occurred: {:?}\nLocation: {:?}\nBacktrace: {:?}",
            panic_info.payload().downcast_ref::<&str>().unwrap_or(&"<panic message not available>"),
            panic_info.location(),
            backtrace
        );
    }));
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    setup_panic_handler();

    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse("https://starknet-mainnet.public.blastapi.io/rpc/v0_7")
            .map_err(AggregateError::Url)?
    ));

    LAST_PROCESSED_BLOCK.store(0, Ordering::SeqCst);
    let (tx, mut rx) = mpsc::channel(100);
    let tx = Arc::new(tx);

    let config = Config::new().expect("Failed to load configuration");
    let mut indexer_service = IndexerService::new(config).await;

    // Single event processor
    let process_handle = task::spawn(async move {
        while let Some((block_number, event)) = rx.recv().await {
            println!("event pop {} {:?}", block_number, event);
            if let Err(err) = handle_event(block_number, event, provider.clone()).await {
                error!("Error processing event at block {}: {:?}", block_number, err);
            }
        }
    });

    let handler = {
        let tx = Arc::clone(&tx);
        move |block_number: u64, event: &Event| {
            println!("\n\nreceived event: {:?} \n\n", event);
            let event_clone = event.clone();  // Still needed to move into async block
            let tx = Arc::clone(&tx);
            tokio::spawn(async move {
                if let Err(e) = tx.send((block_number, event_clone)).await {
                    error!("Failed to send event to processor: {:?}", e);
                }
            });
        }
    };
    
    indexer_service = indexer_service.with_handler(handler);
    
    let shutdown_signal = tokio::signal::ctrl_c();

    tokio::select! {
        result = indexer_service.run_forever() => {
            if let Err(err) = result {
                error!("Indexer service error: {:?}", err);
            }
        }
        _ = shutdown_signal => {
            info!("Received shutdown signal");
        }
    }

    // Wait for processor to complete
    if let Err(e) = process_handle.await {
        error!("Process handle error: {:?}", e);
    }

    Ok(())
}

async fn handle_event(block_number: u64, event: Event, provider: JsonRpcClient<HttpTransport>) -> Result<()> {
    let event_selector = event.keys.first().context("No event selector")?;
    let event_data: Vec<Felt> = event.data.iter().map(apibara_field_as_felt).collect();
    
    match event_selector {
        // selector if selector == &*CREATION_EVENT => {
        //     let creation_event = decode_creation_data(event_data).await?;
        //     info!("Creation event processed at block {}", block_number);
        // }
        selector if selector == &*LAUNCH_EVENT => {
            println!("new Launch event received at: {} \n\n", block_number);
            let launch_event = decode_launch_data(event_data).await?;
            process_launch_event(block_number, launch_event, provider).await?;
        }
        _ => (),
    }
    
    Ok(())
}

async fn process_launch_event(block_number: u64, launch_event: LaunchEvent, provider: JsonRpcClient<HttpTransport>) -> Result<()> {
    if block_number <= LAST_PROCESSED_BLOCK.load(Ordering::Relaxed) {
        return Ok(());
    }

    // let _permit = CONCURRENT_CALLS.acquire().await?;
    let memecoin_address = launch_event.memecoin_address.to_string();
    println!("memecoin_address: {:?}", launch_event.memecoin_address);
    let _provider = provider.clone();
    match get_aggregate_call_data(&memecoin_address, _provider).await {
        Ok(response) => {
            info!("Processed memecoin {} at block {}", memecoin_address, block_number);
            for data in response.iter() {
                info!("{}", data);
            }
            LAST_PROCESSED_BLOCK.store(block_number, Ordering::Relaxed);
        }
        Err(error) => {
            error!("Failed to process {}: {:?}", memecoin_address, error);
            task::spawn(retry_failed_call(memecoin_address, block_number, provider.clone()));
        }
    }
    Ok(())
}

async fn retry_failed_call(address: String, block_number: u64, provider: JsonRpcClient<HttpTransport>) {
    for retry in 0..3 {
        tokio::time::sleep(tokio::time::Duration::from_millis(1000 * (retry + 1) as u64)).await;
        if get_aggregate_call_data(&address, provider.clone()).await.is_ok() {
            info!("Retry succeeded for {} on attempt {}", address, retry + 1);
            LAST_PROCESSED_BLOCK.store(block_number, Ordering::Relaxed);
            return;
        }
    }
    error!("All retries failed for {}", address);
}

async fn decode_creation_data(event_data: Vec<Felt>) -> Result<CreationEvent> {
    CreationEvent::from_starknet_event_data(event_data).context("Parsing Creation Event")
}

async fn decode_launch_data(event_data: Vec<Felt>) -> Result<LaunchEvent> {
    LaunchEvent::from_starknet_event_data(event_data).context("Parsing Launch Event")
}
