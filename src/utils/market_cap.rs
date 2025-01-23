use super::types::ekubo::QuoteResponseApi;

async fn get_ekubo_quote(
    amount: String,
    from_token: &str,
    to_token: &str,
) -> Result<QuoteResponseApi, anyhow::Error> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://quoter-mainnet-api.ekubo.org/{}/{}/{}",
        amount, from_token, to_token
    );
    println!("ekubo quote url: {}", url);
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
    token_address: &str,
) -> Result<(String, String), anyhow::Error> {
    let usdt_address = "0x068f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8";
    let amount = 10u64.pow(6).to_string();

    // Try to get quote with better error handling
    let response = match get_ekubo_quote(amount, &usdt_address, &token_address).await {
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
    let response_total_num: f64 = match response.total_calculated.parse() {
        Ok(num) => num,
        Err(_) => {
            eprintln!("Failed to parse response total: {}", response.total_calculated);
            return Err(anyhow::Error::msg("Failed to parse response total"));
        }
    };

    // Perform the calculation
    let market_cap = total_supply_num / response_total_num;
    let token_price: f64 = 1f64 / response_total_num;

    Ok((token_price.to_string(), market_cap.to_string()))
}


#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_get_ekubo_quote() {
        let amount = "1000000".to_string(); // 1 USDT in micro-units
        let from_token = "0x068f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8"; // USDT address
        let to_token = "0x0388588584bd8c651151f6baf241a85827e7ff0574101f2a8194a3df68a7e2fe"; // Example token

        let result = get_ekubo_quote(amount, from_token, to_token).await;
        
        match result {
            Ok(response) => {
                assert!(!response.total_calculated.is_empty(), "Expected a non-empty total_calculated");
                println!("Received quote: {:?}", response);
            }
            Err(err) => {
                panic!("Failed to get quote: {:?}", err);
            }
        }
    }
}
