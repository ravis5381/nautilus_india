use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub fn sign(self) -> i64 {
        match self {
            Self::Buy => 1,
            Self::Sell => -1,
        }
    }
}

impl fmt::Display for Side {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Buy => formatter.write_str("BUY"),
            Self::Sell => formatter.write_str("SELL"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderStatus {
    Filled,
    Rejected,
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Filled => formatter.write_str("FILLED"),
            Self::Rejected => formatter.write_str("REJECTED"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    pub strategy: String,
    pub instrument_id: String,
    pub quantity: i64,
    pub avg_price: Decimal,
    pub realized_pnl: Decimal,
}

impl Position {
    pub fn new(strategy: impl Into<String>, instrument_id: impl Into<String>) -> Self {
        Self {
            strategy: strategy.into(),
            instrument_id: instrument_id.into(),
            quantity: 0,
            avg_price: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
        }
    }

    pub fn side(&self) -> Option<Side> {
        match self.quantity.signum() {
            1 => Some(Side::Buy),
            -1 => Some(Side::Sell),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OrderRequest {
    pub strategy: String,
    pub instrument_id: String,
    pub side: Side,
    pub quantity: i64,
    pub price: Decimal,
}

#[derive(Clone, Debug)]
pub struct Order {
    pub id: String,
    pub strategy: String,
    pub instrument_id: String,
    pub side: Side,
    pub quantity: i64,
    pub price: Decimal,
    pub status: OrderStatus,
    pub gateway: String,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct Fill {
    pub id: String,
    pub order_id: String,
    pub strategy: String,
    pub instrument_id: String,
    pub side: Side,
    pub quantity: i64,
    pub price: Decimal,
    pub filled_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrder {
    pub strategy: String,
    pub instrument_id: String,
    pub side: Side,
    pub quantity: i64,
    pub price: String,
}

impl CreateOrder {
    pub fn parse(self) -> Result<OrderRequest, String> {
        let strategy = normalize_strategy(&self.strategy)?;
        if self.quantity <= 0 {
            return Err("Quantity must be a positive whole number.".into());
        }
        let price = Decimal::from_str(self.price.trim())
            .map_err(|_| format!("Price {:?} is not a decimal.", self.price))?;
        if price <= Decimal::ZERO {
            return Err("Price must be positive.".into());
        }
        let parsed = nautilus_india_symbology::parse_instrument_id(&self.instrument_id)
            .map_err(|error| error.to_string())?;
        Ok(OrderRequest {
            strategy,
            instrument_id: parsed.instrument_id(),
            side: self.side,
            quantity: self.quantity,
            price,
        })
    }
}

fn normalize_strategy(value: &str) -> Result<String, String> {
    let strategy = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if strategy.is_empty() {
        return Err("Strategy name is required.".into());
    }
    if strategy.chars().count() > 40 {
        return Err("Strategy name must be 40 characters or fewer.".into());
    }
    let allowed = strategy
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ' '));
    if !allowed {
        return Err(
            "Strategy name can use letters, numbers, spaces, hyphens, and underscores.".into(),
        );
    }
    Ok(strategy)
}

pub fn money(value: Decimal) -> String {
    format!("{value:.4}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}
