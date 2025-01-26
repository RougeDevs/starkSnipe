use std::env::VarError;

use starknet::providers::ProviderError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UtilityError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Parsing error: {0}")]
    ParsingError(String),

    #[error("Formatting error: {0}")]
    FormattingError(String),

    #[error("Address validation failed")]
    InvalidAddress,
}

#[derive(Debug, thiserror::Error)]
pub enum CallError {
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("URL parsing error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Contract call failed: {0}")]
    ContractCall(String),

    #[error("Parse error: {0}")]
    Parse(String),
}

#[derive(Error, Debug)]
pub enum TokenError {
    #[error("Environment variable error: {0}")]
    EnvVarError(#[from] VarError),

    #[error("Network request error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Invalid account address: {0}")]
    InvalidAccountError(String),

    #[error("Token parsing error: {0}")]
    ParsingError(String),

    #[error("Balance retrieval error: {0}")]
    BalanceError(String),

    #[error("Market data retrieval error: {0}")]
    MarketDataError(String),
}