// Port of C++ emStocksRec.h / emStocksRec.cpp

use std::fmt;
use std::str::FromStr;

use crate::emCore::emCrossPtr::emCrossPtrList;
use crate::emCore::emRec::{RecStruct, RecValue};
use crate::emCore::emRecRecord::{RecError, Record};

// ─── Interest ────────────────────────────────────────────────────────────────

/// Port of C++ emStocksRec::InterestType + InterestRec.
/// DIVERGED: Rust enum replaces C++ int enum + emEnumRec subclass.
/// Deprecated identifier handling via explicit methods rather than virtual TryStartReading.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Interest {
    High = 0,
    #[default]
    Medium = 1,
    Low = 2,
}

impl fmt::Display for Interest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::High => write!(f, "HIGH"),
            Self::Medium => write!(f, "MEDIUM"),
            Self::Low => write!(f, "LOW"),
        }
    }
}

impl FromStr for Interest {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "HIGH" => Ok(Self::High),
            "MEDIUM" => Ok(Self::Medium),
            "LOW" => Ok(Self::Low),
            _ => Err(format!("unknown interest: {s}")),
        }
    }
}

impl Interest {
    /// C++ buggy deprecated mapping (bugInDeprecatedIdentifiers=true):
    /// "LOW_INTEREST" → High (bug), "HIGH_INTEREST" → Low (bug),
    /// "MEDIUM_INTEREST" → Medium. Case-insensitive.
    pub fn from_deprecated_bugged(s: &str) -> Self {
        let upper = s.to_ascii_uppercase();
        match upper.as_str() {
            "LOW_INTEREST" => Self::High,      // C++ bug: swapped
            "HIGH_INTEREST" => Self::Low,      // C++ bug: swapped
            "MEDIUM_INTEREST" => Self::Medium,
            _ => Self::Medium,
        }
    }

    /// Normal deprecated mapping (no bug):
    /// "LOW_INTEREST" → Low, "HIGH_INTEREST" → High. Case-insensitive.
    pub fn from_deprecated_normal(s: &str) -> Self {
        let upper = s.to_ascii_uppercase();
        match upper.as_str() {
            "LOW_INTEREST" => Self::Low,
            "HIGH_INTEREST" => Self::High,
            "MEDIUM_INTEREST" => Self::Medium,
            _ => Self::Medium,
        }
    }

    /// Try to parse from a rec identifier (lowercase from RecStruct).
    /// Tries canonical names first, then deprecated-with-bug (matching C++ constructor).
    fn from_rec_ident(s: &str) -> Self {
        let upper = s.to_ascii_uppercase();
        if let Ok(interest) = Interest::from_str(&upper) {
            return interest;
        }
        Interest::from_deprecated_bugged(&upper)
    }
}

// ─── StockRec ────────────────────────────────────────────────────────────────

/// Port of C++ emStocksRec::StockRec.
/// DIVERGED: Rust struct fields use snake_case. Method names preserve C++ names.
#[derive(Default)]
pub struct StockRec {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub wkn: String,
    pub isin: String,
    pub country: String,
    pub sector: String,
    pub collection: String,
    pub comment: String,
    pub owning_shares: bool,
    pub own_shares: String,
    pub trade_price: String,
    pub trade_date: String,
    pub prices: String,
    pub last_price_date: String,
    pub desired_price: String,
    pub expected_dividend: String,
    pub inquiry_date: String,
    pub interest: Interest,
    pub web_pages: Vec<String>,
    cross_ptr_list: emCrossPtrList,
}

impl Clone for StockRec {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            symbol: self.symbol.clone(),
            wkn: self.wkn.clone(),
            isin: self.isin.clone(),
            country: self.country.clone(),
            sector: self.sector.clone(),
            collection: self.collection.clone(),
            comment: self.comment.clone(),
            owning_shares: self.owning_shares,
            own_shares: self.own_shares.clone(),
            trade_price: self.trade_price.clone(),
            trade_date: self.trade_date.clone(),
            prices: self.prices.clone(),
            last_price_date: self.last_price_date.clone(),
            desired_price: self.desired_price.clone(),
            expected_dividend: self.expected_dividend.clone(),
            inquiry_date: self.inquiry_date.clone(),
            interest: self.interest,
            web_pages: self.web_pages.clone(),
            cross_ptr_list: emCrossPtrList::new(),
        }
    }
}

impl fmt::Debug for StockRec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StockRec")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("symbol", &self.symbol)
            .field("wkn", &self.wkn)
            .field("isin", &self.isin)
            .field("country", &self.country)
            .field("sector", &self.sector)
            .field("collection", &self.collection)
            .field("comment", &self.comment)
            .field("owning_shares", &self.owning_shares)
            .field("own_shares", &self.own_shares)
            .field("trade_price", &self.trade_price)
            .field("trade_date", &self.trade_date)
            .field("prices", &self.prices)
            .field("last_price_date", &self.last_price_date)
            .field("desired_price", &self.desired_price)
            .field("expected_dividend", &self.expected_dividend)
            .field("inquiry_date", &self.inquiry_date)
            .field("interest", &self.interest)
            .field("web_pages", &self.web_pages)
            .finish()
    }
}

impl StockRec {
    /// Expose cross-pointer list for linking. Corresponds to C++ LinkCrossPtr.
    pub fn LinkCrossPtr(&mut self) -> &mut emCrossPtrList {
        &mut self.cross_ptr_list
    }
}

impl PartialEq for StockRec {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.symbol == other.symbol
            && self.wkn == other.wkn
            && self.isin == other.isin
            && self.country == other.country
            && self.sector == other.sector
            && self.collection == other.collection
            && self.comment == other.comment
            && self.owning_shares == other.owning_shares
            && self.own_shares == other.own_shares
            && self.trade_price == other.trade_price
            && self.trade_date == other.trade_date
            && self.prices == other.prices
            && self.last_price_date == other.last_price_date
            && self.desired_price == other.desired_price
            && self.expected_dividend == other.expected_dividend
            && self.inquiry_date == other.inquiry_date
            && self.interest == other.interest
            && self.web_pages == other.web_pages
    }
}

impl Record for StockRec {
    fn from_rec(rec: &RecStruct) -> Result<Self, RecError> {
        let get_str = |name: &str| -> String {
            rec.get_str(name).unwrap_or("").to_string()
        };

        let interest = if let Some(ident) = rec.get_ident("Interest") {
            Interest::from_rec_ident(ident)
        } else {
            Interest::default()
        };

        let web_pages = if let Some(arr) = rec.get_array("WebPages") {
            arr.iter()
                .filter_map(|v| {
                    if let RecValue::Str(s) = v {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok(Self {
            id: get_str("Id"),
            name: get_str("Name"),
            symbol: get_str("Symbol"),
            wkn: get_str("WKN"),
            isin: get_str("ISIN"),
            country: get_str("Country"),
            sector: get_str("Sector"),
            collection: get_str("Collection"),
            comment: get_str("Comment"),
            owning_shares: rec.get_bool("OwningShares").unwrap_or(false),
            own_shares: get_str("OwnShares"),
            trade_price: get_str("TradePrice"),
            trade_date: get_str("TradeDate"),
            prices: get_str("Prices"),
            last_price_date: get_str("LastPriceDate"),
            desired_price: get_str("DesiredPrice"),
            expected_dividend: get_str("ExpectedDividend"),
            inquiry_date: get_str("InquiryDate"),
            interest,
            web_pages,
            cross_ptr_list: emCrossPtrList::new(),
        })
    }

    fn to_rec(&self) -> RecStruct {
        let mut rec = RecStruct::new();
        rec.set_str("Id", &self.id);
        rec.set_str("Name", &self.name);
        rec.set_str("Symbol", &self.symbol);
        rec.set_str("WKN", &self.wkn);
        rec.set_str("ISIN", &self.isin);
        rec.set_str("Country", &self.country);
        rec.set_str("Sector", &self.sector);
        rec.set_str("Collection", &self.collection);
        rec.set_str("Comment", &self.comment);
        rec.set_bool("OwningShares", self.owning_shares);
        rec.set_str("OwnShares", &self.own_shares);
        rec.set_str("TradePrice", &self.trade_price);
        rec.set_str("TradeDate", &self.trade_date);
        rec.set_str("Prices", &self.prices);
        rec.set_str("LastPriceDate", &self.last_price_date);
        rec.set_str("DesiredPrice", &self.desired_price);
        rec.set_str("ExpectedDividend", &self.expected_dividend);
        rec.set_str("InquiryDate", &self.inquiry_date);
        rec.set_ident("Interest", &self.interest.to_string());
        rec.SetValue(
            "WebPages",
            RecValue::Array(
                self.web_pages
                    .iter()
                    .map(|s| RecValue::Str(s.clone()))
                    .collect(),
            ),
        );
        rec
    }

    fn SetToDefault(&mut self) {
        *self = Self::default();
    }

    fn IsSetToDefault(&self) -> bool {
        *self == Self::default()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interest_from_str_canonical() {
        assert_eq!(Interest::from_str("HIGH"), Ok(Interest::High));
        assert_eq!(Interest::from_str("MEDIUM"), Ok(Interest::Medium));
        assert_eq!(Interest::from_str("LOW"), Ok(Interest::Low));
    }

    #[test]
    fn interest_from_str_deprecated_with_bug() {
        assert_eq!(
            Interest::from_deprecated_bugged("LOW_INTEREST"),
            Interest::High
        );
        assert_eq!(
            Interest::from_deprecated_bugged("HIGH_INTEREST"),
            Interest::Low
        );
        assert_eq!(
            Interest::from_deprecated_bugged("MEDIUM_INTEREST"),
            Interest::Medium
        );
    }

    #[test]
    fn interest_from_str_deprecated_no_bug() {
        assert_eq!(
            Interest::from_deprecated_normal("LOW_INTEREST"),
            Interest::Low
        );
        assert_eq!(
            Interest::from_deprecated_normal("HIGH_INTEREST"),
            Interest::High
        );
    }

    #[test]
    fn interest_display() {
        assert_eq!(Interest::High.to_string(), "HIGH");
        assert_eq!(Interest::Medium.to_string(), "MEDIUM");
        assert_eq!(Interest::Low.to_string(), "LOW");
    }

    #[test]
    fn stock_rec_default() {
        let rec = StockRec::default();
        assert_eq!(rec.id, "");
        assert_eq!(rec.interest, Interest::Medium);
        assert!(rec.web_pages.is_empty());
    }

    #[test]
    fn stock_rec_record_round_trip() {
        let mut rec = StockRec::default();
        rec.id = "42".to_string();
        rec.name = "Test Stock".to_string();
        rec.symbol = "TST".to_string();
        rec.interest = Interest::High;
        rec.web_pages = vec!["https://example.com".to_string()];

        let serialized = rec.to_rec();
        let deserialized = StockRec::from_rec(&serialized).unwrap();

        assert_eq!(deserialized.id, "42");
        assert_eq!(deserialized.name, "Test Stock");
        assert_eq!(deserialized.symbol, "TST");
        assert_eq!(deserialized.interest, Interest::High);
        assert_eq!(deserialized.web_pages, vec!["https://example.com"]);
    }
}
