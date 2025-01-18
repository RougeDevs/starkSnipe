use super::types::ekubo::QuoteResponseApi;

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
    total_supply: &str,
    symbol: &str,
) -> Result<(String, String), anyhow::Error> {
    let amount = 10u64.pow(6).to_string();

    // Try to get quote with better error handling
    let response = match get_ekubo_quote(amount, "USDT", &symbol).await {
        Ok(response) => {
            // println!("Received quote: {:?}", response);
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
    let token_price: f64 = 1f64 / response_total_num;

    Ok((token_price.to_string(), market_cap.to_string()))
}
