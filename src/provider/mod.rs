// use std::time::{self, Duration};

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};
use starknet::{
    core::
        types::{EventFilter, EventsPage}
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
    pub fn new(url: Url, options: StarknetProviderOptions) {
        let mut transport = HttpTransport::new(url);

        // Set headers
        for (key, value) in options.headers.iter() {
            let key = key.to_string();
            match value.to_str() {
                Ok(value_str) => {
                    transport.add_header(key, value_str.to_string());
                }

                Err(e) => {
                    println!("Error converting header value to string: {}", e);
                }
            }
        }

        let provider = JsonRpcClient::new(transport);

        Ok::<Monitor, StarknetProviderError>(Self {
            provider,
            interval: Duration::from_secs(15)
        });
    }

    pub fn set_poll_interval(&mut self, interval: Duration) {
        self.interval = interval;
    }

    pub async fn listen_for_events(&self, filter: EventFilter, mut callback: impl FnMut(EventsPage) -> ()) -> Result<(), StarknetProviderError> {
        let mut interval = time::interval(self.interval);
        let continuation_token = None;
        loop {
            interval.tick().await;

            match self.provider.get_events(filter.clone(), continuation_token.clone(), 1000).await {
                Ok(events) => {
                    if events.events.len() != 0 {
                        callback(events);
                    }
                }
                Err(e) => {
                    println!("Error fetching events: {:?}", e);
                    continue;
                }
            }
        }
    }
}