use std::ops::{Add, Sub, Mul, Div};
use std::cmp::Ordering;
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Zero};
use num_integer::Integer;
use std::cmp::min;
use thiserror::Error;

/// Represents errors that can occur when working with Fraction
#[derive(Error, Debug)]
pub enum FractionError {
    #[error("Invalid fraction: {0}")]
    InvalidFraction(String),
    #[error("Division by zero")]
    DivisionByZero,
    #[error("Parsing error: {0}")]
    ParseError(String),
}

/// Rounding modes for decimal operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Rounding {
    RoundDown,
    RoundHalfUp,
    RoundUp,
}

/// A fraction represented by a numerator and denominator using arbitrary-precision integers
#[derive(Debug, Clone)]
pub struct Fraction {
    pub numerator: BigInt,
    pub denominator: BigInt,
}

impl Fraction {
    /// Creates a new Fraction from numerator and denominator
    /// 
    /// # Arguments
    /// * `numerator` - The numerator of the fraction
    /// * `denominator` - The denominator of the fraction (defaults to 1)
    pub fn new<T: Into<BigInt>>(numerator: T, denominator: Option<T>) -> Result<Self, FractionError> {
        let num = numerator.into();
        let den = match denominator {
            Some(d) => d.into(),
            None => BigInt::from(1),
        };

        if den == BigInt::from(0) {
            return Err(FractionError::DivisionByZero);
        }

        Ok(Self {
            numerator: num,
            denominator: den,
        })
    }

    /// Gets the quotient (floor division) of the fraction
    pub fn quotient(&self) -> BigInt {
        &self.numerator / &self.denominator
    }

    /// Gets the remainder after floor division
    pub fn remainder(&self) -> Fraction {
        Fraction {
            numerator: &self.numerator % &self.denominator,
            denominator: self.denominator.clone(),
        }
    }

    /// Inverts the fraction (swaps numerator and denominator)
    pub fn invert(&self) -> Result<Fraction, FractionError> {
        if self.numerator == BigInt::from(0) {
            return Err(FractionError::DivisionByZero);
        }
        
        Ok(Fraction {
            numerator: self.denominator.clone(),
            denominator: self.numerator.clone(),
        })
    }

    pub fn to_formatted_string(&self) -> Result<String, Box<dyn std::error::Error>> {
        // Handle zero numerator case
        if self.numerator.is_zero() {
            return Ok("0".to_string());
        }

        let mut numerator = self.numerator.clone();
        let denominator = self.denominator.clone();
        
        // Calculate the integer result with extra precision for rounding
        let precision = 18; // Use high precision for calculation
        let scale = BigUint::from(10u64).pow(precision);
        numerator *= BigInt::from(scale);
        let (quotient, remainder) = numerator.div_rem(&denominator);
        
        // Round up if necessary
        let remainder_as_biguint: BigUint = remainder.to_biguint().unwrap();
        let rounded = if remainder_as_biguint * BigUint::from(2u64) >= denominator.to_biguint().unwrap() {
            quotient + BigInt::from(1u64)
        } else {
            quotient
        };

        // Convert to string and handle decimal point placement
        let mut str_value = rounded.to_string();
        
        // Pad with leading zeros if necessary
        while str_value.len() <= precision.try_into().unwrap() {
            str_value.insert(0, '0');
        }

        // Insert decimal point
        let decimal_pos = str_value.len() - precision as usize;
        let int_part = &str_value[..decimal_pos];
        let frac_part = &str_value[decimal_pos..];

        // Remove trailing zeros after decimal and handle formatting
        let mut formatted = if frac_part.chars().all(|c| c == '0') {
            int_part.to_string()
        } else {
            format!("{}.{}", int_part, frac_part.trim_end_matches('0'))
        };

        // Add thousand separators to the integer part
        let dot_pos = formatted.find('.');
        let int_end = dot_pos.unwrap_or(formatted.len());
        let mut with_separators = String::new();
        let int_chars: Vec<char> = formatted[..int_end].chars().collect();
        
        for (i, &c) in int_chars.iter().enumerate() {
            if i > 0 && (int_chars.len() - i) % 3 == 0 {
                with_separators.push(',');
            }
            with_separators.push(c);
        }

        if let Some(dot_pos) = dot_pos {
            with_separators.push_str(&formatted[dot_pos..]);
        }

        Ok(with_separators)
    }

    pub fn to_significant_digits(&self, digits: usize, rounding: Rounding) -> Result<String, Box<dyn std::error::Error>> {
        let formatted = self.to_formatted_string()?;
        if formatted == "0" {
            return Ok(formatted);
        }

        // Find the first non-zero digit
        let first_non_zero = formatted
            .chars()
            .position(|c| c != '0' && c != '.' && c != ',')
            .unwrap_or(0);

        // Count significant digits from the first non-zero digit
        let mut count = 0;
        let mut result = String::new();
        let mut seen_decimal = false;

        for c in formatted.chars() {
            match c {
                '.' => {
                    seen_decimal = true;
                    result.push(c);
                }
                ',' => result.push(c),
                '0'..='9' => {
                    if count < digits || !seen_decimal {
                        if c != '0' || count > 0 {
                            count += 1;
                        }
                        result.push(c);
                    }
                }
                _ => {}
            }
        }

        // Handle rounding if necessary
        if rounding == Rounding::RoundDown {
            while result.ends_with('0') && seen_decimal {
                result.pop();
            }
            if result.ends_with('.') {
                result.pop();
            }
        }

        Ok(result)
    }
}

// Implement basic arithmetic operations
impl Add for Fraction {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        if self.denominator == other.denominator {
            return Self {
                numerator: self.numerator + other.numerator,
                denominator: self.denominator,
            };
        }

        Self {
            numerator: self.numerator * &other.denominator + other.numerator * &self.denominator,
            denominator: self.denominator * other.denominator,
        }
    }
}

impl Sub for Fraction {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        if self.denominator == other.denominator {
            return Self {
                numerator: self.numerator - other.numerator,
                denominator: self.denominator,
            };
        }

        Self {
            numerator: self.numerator * &other.denominator - other.numerator * &self.denominator,
            denominator: self.denominator * other.denominator,
        }
    }
}

impl Mul for Fraction {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self {
            numerator: self.numerator * other.numerator,
            denominator: self.denominator * other.denominator,
        }
    }
}

impl Div for Fraction {
    type Output = Result<Self, FractionError>;

    fn div(self, other: Self) -> Result<Self, FractionError> {
        if other.numerator == BigInt::from(0) {
            return Err(FractionError::DivisionByZero);
        }

        Ok(Self {
            numerator: self.numerator * other.denominator,
            denominator: self.denominator * other.numerator,
        })
    }
}

// Implement comparison operations
impl PartialEq for Fraction {
    fn eq(&self, other: &Self) -> bool {
        self.numerator.clone() * &other.denominator == other.numerator.clone() * &self.denominator
    }
}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (self.numerator.clone() * &other.denominator)
            .partial_cmp(&(other.numerator.clone() * &self.denominator))
    }
}