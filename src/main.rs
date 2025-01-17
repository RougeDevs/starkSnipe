use apibara_core::starknet::v1alpha2::Event;
use dotenv::dotenv;
use kanshi::{config::Config, dna::IndexerService};
use tokio::sync::mpsc;
use tokio::task;
use anyhow::Result;

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


async fn process_event(event: Event) {
    // Add your event processing logic here
    // For example:
    match event {
        // Add pattern matching for different event types
        _ => {
            // Default processing
            println!("Processing event: {:?}", event);
        }
    }
}
