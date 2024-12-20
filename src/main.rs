use std::time::Duration;

use provider::{Monitor, StarknetProviderError, StarknetProviderOptions};
use reqwest::header::{HeaderMap, HeaderValue};
use starknet::core::types::{BlockId, EventFilter, Felt};
use tokio::runtime::Builder;
use url::Url;

mod provider;

fn main() -> Result<(), StarknetProviderError> {
    let rt = Builder::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
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
    })
}