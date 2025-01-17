use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
pub struct PoolKeyResponse {
    token0: String,
    token1: String,
    fee: String,
    tick_spacing: u64,
    extension: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RouteResponse {
    pool_key: PoolKeyResponse,
    sqrt_ratio_limit: String,
    skip_ahead: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SplitResponse {
    amount: String,
    #[serde(rename = "specifiedAmount")]
    specified_amount: String,
    route: Vec<RouteResponse>,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct QuoteResponseApi {
    total: String,
    splits: Vec<SplitResponse>,
}

async fn get_ekubo_quote(
    amount: String,
    from_token: &str,
    to_token: &str,
) -> Result<QuoteResponseApi, anyhow::Error> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://mainnet-api.ekubo.org/quote/{}/{}/{}",
        amount, from_token, to_token
    );

    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(10)) // 10-second timeout
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        return Err(anyhow::Error::msg(format!(
            "API call failed with status: {}",
            status
        )));
    }

    let quote: QuoteResponseApi = response.json().await?;
    Ok(quote)
}

pub async fn calculate_market_cap(
    total_supply: String,
    symbol: String,
) -> Result<String, anyhow::Error> {
    let amount = 10u64.pow(6).to_string();

    // Try to get quote with better error handling
    let response = match get_ekubo_quote(amount, "USDT", &symbol).await {
        Ok(response) => {
            println!("Received quote: {:?}", response);
            response
        }
        Err(err) => {
            eprintln!("Error while getting quote: {:?}", err);
            return Err(anyhow::Error::msg(err.to_string()));
        }
    };
    let total_supply_num: f64 = match total_supply.parse() {
        Ok(num) => num,
        Err(_) => {
            eprintln!("Failed to parse total_supply: {}", total_supply);
            return Err(anyhow::Error::msg("Failed to parse total_supply"));
        }
    };

    // Parse response total safely
    let response_total_num: f64 = match response.total.parse() {
        Ok(num) => num,
        Err(_) => {
            eprintln!("Failed to parse response total: {}", response.total);
            return Err(anyhow::Error::msg("Failed to parse response total"));
        }
    };

    // Perform the calculation
    let market_cap = total_supply_num / response_total_num;

    Ok(market_cap.to_string())
}
