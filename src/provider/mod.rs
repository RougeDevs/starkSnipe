// use std::time::{self, Duration};

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};
use starknet::{
    core::
        types::{BlockId, EventFilter, EventsPage}
    ,
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient}, Provider}
};
use tokio::time;
use url::Url;

#[derive(Debug, Clone)]
pub struct StarknetProviderOptions {
    /// Request timeout.
    pub timeout: Duration,
    /// Request headers.
    pub headers: HeaderMap<HeaderValue>,
}

#[derive(Debug)]
pub enum StarknetProviderError {
    Request,
    Timeout,
    NotFound,
    Configuration,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub from_address: String,
    pub block_number: u64,
    pub block_hash: String,
    pub transaction_hash: String,
    pub keys: Vec<String>,
    pub data: Vec<String>,
}

#[derive(Debug)]
pub struct Monitor {
    provider: JsonRpcClient<HttpTransport>,
    interval: Duration,
}

impl Monitor {
    pub fn new(url: Url, options: StarknetProviderOptions) -> Result<Self, StarknetProviderError> {
        let mut transport = HttpTransport::new(url);
        println!("Initializing monitor with URL: {:?}", transport);
        // Set headers
        for (key, value) in options.headers.iter() {
            let key = key.to_string();
            match value.to_str() {
                Ok(value_str) => {
                    transport.add_header(key, value_str.to_string());
                }

                Err(e) => {
                    println!("Error converting header value to string: {}", e);
                    return Err(StarknetProviderError::Configuration);
                }
            }
        }

        let provider = JsonRpcClient::new(transport);

        Ok(Self {
            provider,
            interval: Duration::from_secs(15)
        })
    }

    pub fn set_poll_interval(&mut self, interval: Duration) {
        self.interval = interval;
    }

    pub async fn listen_for_events(&self, filter: EventFilter, mut callback: impl FnMut(EventsPage) -> ()) -> Result<(), StarknetProviderError> {
        let mut interval = time::interval(self.interval);
        let mut continuation_token = None;
        let mut last_processed_block = match filter.from_block {
            Some(BlockId::Number(n)) => n,
            _ => 0,
        };
    
        // First, catch up with historical events
        loop {
            println!("Fetching historical events from block {} with token: {:?}", last_processed_block, continuation_token);
            match self.provider.get_events(filter.clone(), continuation_token.clone(), 1000).await {
                Ok(events) => {
                    if events.events.is_empty() && events.continuation_token.is_none() {
                        // No more historical events to process
                        println!("Caught up with historical events");
                        break;
                    }
    
                    if !events.events.is_empty() {
                        // Update last processed block
                        if let Some(last_event) = events.events.last() {
                            last_processed_block = last_event.block_number.unwrap() as u64;
                        }
                        callback(events.clone());
                    }
    
                    // Update continuation token for next request
                    continuation_token = events.continuation_token;
                    
                    // If no continuation token, we've processed all events
                    if continuation_token.is_none() {
                        break;
                    }
                }
                Err(e) => {
                    println!("Error fetching historical events: {:?}", e);
                    // Short delay before retry
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            }
        }
    
        // Now start listening for new events
        println!("Starting to listen for new events from block {}", last_processed_block);
        let mut new_filter = filter.clone();
        loop {
            interval.tick().await;
    
            // Update filter to only look for new events
            new_filter.from_block = Some(BlockId::Number(last_processed_block + 1));
            
            match self.provider.get_events(new_filter.clone(), None, 1000).await {
                Ok(events) => {
                    if !events.events.is_empty() {
                        // Update last processed block
                        if let Some(last_event) = events.events.last() {
                            last_processed_block = last_event.block_number.unwrap() as u64;
                        }
                        callback(events);
                    }
                }
                Err(e) => {
                    println!("Error fetching new events: {:?}", e);
                    // Short delay before retry
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            }
        }
    }
}