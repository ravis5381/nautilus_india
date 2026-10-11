//! Canonical instrument-id grammar shared with `nautilus_india.symbols`.
//!
//! The Python package and the desk both parse ids through this crate. It does
//! not link Nautilus model types: those objects are constructed in Python from
//! the installed `nautilus_trader` wheel.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndiaSymbol {
    pub root: String,
    pub venue: String,
    pub product: Product,
    pub expiry: Option<String>,
    pub strike: Option<String>,
    pub option_kind: Option<OptionKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Product {
    Cash,
    Future,
    Option,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionKind {
    Call,
    Put,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

impl IndiaSymbol {
    /// Canonical `{symbol}.{venue}` id, including a hyphenated root such as `BAJAJ-AUTO`.
    pub fn instrument_id(&self) -> String {
        match self.product {
            Product::Cash => format!("{}.{}", self.root, self.venue),
            Product::Future => {
                let expiry = compact_expiry(self.expiry.as_deref());
                format!("{}-{expiry}-FUT.{}", self.root, self.venue)
            }
            Product::Option => {
                let expiry = compact_expiry(self.expiry.as_deref());
                let strike = self.strike.as_deref().unwrap_or("");
                let kind = match self.option_kind {
                    Some(OptionKind::Call) => "CE",
                    Some(OptionKind::Put) => "PE",
                    None => "",
                };
                format!("{}-{expiry}-{strike}-{kind}.{}", self.root, self.venue)
            }
        }
    }
}

fn compact_expiry(expiry: Option<&str>) -> String {
    expiry.unwrap_or("").replace('-', "")
}

const VENUES: &[&str] = &["NSE", "BSE", "NFO", "BFO", "CDS", "MCX"];

pub fn parse_instrument_id(value: &str) -> Result<IndiaSymbol, ParseError> {
    let text = value.trim().to_ascii_uppercase();
    if text.is_empty() || text.chars().any(char::is_whitespace) {
        return Err(ParseError(format!("Invalid instrument id {value:?}")));
    }
    let (symbol, venue) = text
        .rsplit_once('.')
        .ok_or_else(|| ParseError(format!("Invalid instrument id {value:?}")))?;
    if symbol.is_empty() || !VENUES.contains(&venue) {
        return Err(ParseError(format!("Invalid instrument id {value:?}")));
    }

    if let Some(parsed) = split_future(symbol)? {
        let (root, expiry) = parsed;
        validate_root(&root)?;
        return Ok(IndiaSymbol {
            root,
            venue: venue.to_owned(),
            product: Product::Future,
            expiry: Some(expiry),
            strike: None,
            option_kind: None,
        });
    }
    if let Some(parsed) = split_option(symbol)? {
        let (root, expiry, strike, kind) = parsed;
        validate_root(&root)?;
        return Ok(IndiaSymbol {
            root,
            venue: venue.to_owned(),
            product: Product::Option,
            expiry: Some(expiry),
            strike: Some(strike),
            option_kind: Some(kind),
        });
    }
    validate_root(symbol)?;
    Ok(IndiaSymbol {
        root: symbol.to_owned(),
        venue: venue.to_owned(),
        product: Product::Cash,
        expiry: None,
        strike: None,
        option_kind: None,
    })
}

fn split_future(symbol: &str) -> Result<Option<(String, String)>, ParseError> {
    let Some((root, tail)) = symbol.rsplit_once('-') else {
        return Ok(None);
    };
    if tail != "FUT" {
        return Ok(None);
    }
    let Some((underlying, expiry)) = root.rsplit_once('-') else {
        return Ok(None);
    };
    if !is_digits(expiry, 8) || underlying.is_empty() {
        return Ok(None);
    }
    let expiry = parse_expiry(expiry)?;
    Ok(Some((underlying.to_owned(), expiry)))
}

fn split_option(symbol: &str) -> Result<Option<(String, String, String, OptionKind)>, ParseError> {
    let Some((body, side)) = symbol.rsplit_once('-') else {
        return Ok(None);
    };
    let kind = match side {
        "CE" => OptionKind::Call,
        "PE" => OptionKind::Put,
        _ => return Ok(None),
    };
    let Some((root_and_expiry, strike)) = body.rsplit_once('-') else {
        return Ok(None);
    };
    if !is_strike(strike) {
        return Ok(None);
    }
    let Some((underlying, expiry)) = root_and_expiry.rsplit_once('-') else {
        return Ok(None);
    };
    if !is_digits(expiry, 8) || underlying.is_empty() {
        return Ok(None);
    }
    let expiry = parse_expiry(expiry)?;
    Ok(Some((
        underlying.to_owned(),
        expiry,
        normalize_strike(strike),
        kind,
    )))
}

fn validate_root(root: &str) -> Result<(), ParseError> {
    let pieces: Vec<&str> = root.split('-').collect();
    if pieces.is_empty() || pieces.iter().any(|piece| piece.is_empty()) {
        return Err(ParseError(
            "Instrument symbol contains an empty component".into(),
        ));
    }
    for piece in pieces {
        if !piece
            .chars()
            .all(|char| char.is_ascii_uppercase() || char.is_ascii_digit() || char == '&')
        {
            return Err(ParseError(format!(
                "Unsupported character in symbol component {piece:?}"
            )));
        }
    }
    Ok(())
}

fn is_digits(value: &str, len: usize) -> bool {
    value.len() == len && value.chars().all(|char| char.is_ascii_digit())
}

fn parse_expiry(value: &str) -> Result<String, ParseError> {
    format!("{}-{}-{}", &value[0..4], &value[4..6], &value[6..8])
        .parse::<CalendarDate>()
        .map(|date| date.0)
        .map_err(|_| ParseError(format!("Invalid expiry date {value}")))
}

fn is_strike(value: &str) -> bool {
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or("");
    let fraction = parts.next();
    whole.chars().all(|char| char.is_ascii_digit())
        && !whole.is_empty()
        && fraction
            .map(|text| !text.is_empty() && text.chars().all(|char| char.is_ascii_digit()))
            .unwrap_or(true)
        && parts.next().is_none()
}

fn normalize_strike(value: &str) -> String {
    match value.split_once('.') {
        Some((whole, fraction)) => {
            let trimmed = fraction.trim_end_matches('0');
            if trimmed.is_empty() {
                whole.to_owned()
            } else {
                format!("{whole}.{trimmed}")
            }
        }
        None => value.to_owned(),
    }
}

/// Enough date checking to reject impossible calendar days without a datetime crate.
struct CalendarDate(String);

impl std::str::FromStr for CalendarDate {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split('-');
        let year: i32 = parts.next().ok_or(())?.parse().map_err(|_| ())?;
        let month: u32 = parts.next().ok_or(())?.parse().map_err(|_| ())?;
        let day: u32 = parts.next().ok_or(())?.parse().map_err(|_| ())?;
        if parts.next().is_some() || !(1..=12).contains(&month) {
            return Err(());
        }
        let max_day = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if is_leap(year) => 29,
            2 => 28,
            _ => return Err(()),
        };
        if (1..=max_day).contains(&day) {
            Ok(Self(value.to_owned()))
        } else {
            Err(())
        }
    }
}

fn is_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cash_future_and_option_ids() {
        let cash = parse_instrument_id("bajaj-auto.nse").unwrap();
        assert_eq!(cash.root, "BAJAJ-AUTO");
        assert_eq!(cash.product, Product::Cash);

        let future = parse_instrument_id("NIFTY-20260625-FUT.NFO").unwrap();
        assert_eq!(future.expiry.as_deref(), Some("2026-06-25"));
        assert_eq!(future.product, Product::Future);

        let option = parse_instrument_id("NIFTY-20260625-25000.50-PE.NFO").unwrap();
        assert_eq!(option.strike.as_deref(), Some("25000.5"));
        assert_eq!(option.option_kind, Some(OptionKind::Put));
    }

    #[test]
    fn rejects_an_impossible_expiry() {
        assert!(parse_instrument_id("NIFTY-20260231-FUT.NFO").is_err());
    }

    #[test]
    fn canonicalizes_a_hyphenated_option_root() {
        let option = parse_instrument_id("bajaj-auto-20260625-8500-ce.nfo").unwrap();
        assert_eq!(option.instrument_id(), "BAJAJ-AUTO-20260625-8500-CE.NFO");
    }
}
