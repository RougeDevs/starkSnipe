use regex::Regex;
use serde_json::json;

use crate::utils::error::UtilityError;
pub fn is_valid_starknet_address(address: &str) -> Result<bool, UtilityError> {
    let re = Regex::new(r"^0x[0-9a-fA-F]{50,64}$").map_err(|_| UtilityError::InvalidAddress)?;
    Ok(re.is_match(address))
}

pub fn calculate_team_allocation(
    total_supply: String,
    total_team_allocation: String,
) -> Result<std::string::String, UtilityError> {
    let parsed_total_supply = format_large_number(&total_supply)?
        .parse::<f64>()
        .map_err(|_| UtilityError::FormattingError("Total supply parsing failed".to_string()))?;
    let parsed_team_allocation = format_large_number(&total_team_allocation)?
        .parse::<f64>()
        .map_err(|_| UtilityError::FormattingError("Team allocation parsing failed".to_string()))?;

    let percentage_team_allocation = (parsed_team_allocation * 100.0) / parsed_total_supply;

    Ok(format!("{:.2}", percentage_team_allocation))
}

pub fn format_large_number(input: &str) -> Result<String, UtilityError> {
    // Validate input is numeric
    if !input.chars().all(|c| c.is_digit(10)) {
        return Err(UtilityError::InvalidInput("Must contain only digits".to_string()));
    }

    let input_len = input.len();

    // If input is less than 18 digits, we need to add decimal places
    if input_len < 18 {
        let zeros_needed = 18 - input_len;
        let mut result = "0.".to_string();
        // Add necessary leading zeros
        for _ in 0..zeros_needed {
            result.push('0');
        }
        result.push_str(input.trim_start_matches('0'));
        if result == "0." {
            return Ok("0".to_string());
        }
        return Ok(result
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string());
    }

    // If input is exactly 18 digits, result is 1
    if input_len == 18 {
        return Ok("1".to_string());
    }

    // If input is more than 18 digits, we need to place a decimal point
    let decimal_position = input_len - 18;
    let mut result = input[0..decimal_position].to_string();
    let fraction = &input[decimal_position..];

    if fraction != "000000000000000000" {
        result.push('.');
        result.push_str(fraction.trim_end_matches('0'));
    }

    // Remove leading zeros and handle special case
    result = result.trim_start_matches('0').to_string();
    if result.is_empty() || result.starts_with('.') {
        result = format!("0{}", result);
    }

    // Remove trailing decimal if it exists
    if result.ends_with('.') {
        result.pop();
    }

    Ok(result)
}

pub fn format_short_address(address: &str) -> String {
    if address.len() > 8 {
        format!("{}...{}", &address[..6], &address[address.len() - 4..])
    } else {
        address.to_string()
    }
}

pub fn format_number(num_str: &str) -> Result<String, UtilityError> {
    // Parse the string to f64
    let num = match num_str.parse::<f64>() {
        Ok(n) => n,
        Err(_) => return Err(UtilityError::InvalidInput("Invalid number format".to_string())),
    };

    // Define the thresholds and their corresponding suffixes
    let billion = 1_000_000_000.0;
    let million = 1_000_000.0;
    let thousand = 1_000.0;

    let (value, suffix) = if num >= billion {
        (num / billion, "B")
    } else if num >= million {
        (num / million, "M")
    } else if num >= thousand {
        (num / thousand, "K")
    } else {
        (num, "")
    };

    // Format with up to 2 decimal places, removing trailing zeros
    let formatted = format!("{:.2}", value)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string();

    Ok(format!("{}{}", formatted, suffix))
}

// Helper functions for formatting
pub fn format_price(price: String) -> String {
    format!("{:.2}", price)
}

pub fn format_percentage(value_str: String) -> String {
    // Try to parse the string as f64
    match value_str.parse::<f64>() {
        Ok(value) => {
            format!("{:.1}", value)
        }
        Err(_) => {
            eprintln!("Failed to parse percentage string: {}", value_str);
            value_str
        }
    }
}