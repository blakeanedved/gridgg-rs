//! Rust types for the custom GraphQL scalars of the GRID schemas.
//!
//! | Scalar | Rust type | Schemas |
//! |---|---|---|
//! | `DateTime` | [`jiff::Timestamp`] | Central Data, Series State |
//! | `Date` | [`jiff::civil::Date`] | Central Data |
//! | `Duration` | [`jiff::Span`] | Series State, Stats Feed |
//! | `Decimal` | [`rust_decimal::Decimal`] | Central Data |
//! | `Url` | [`url::Url`] | Central Data |
//! | `HexColor` | [`HexColor`] | Central Data |
//! | `Version` | [`Version`] | Series State |
//!
//! All values travel as ISO 8601 (dates, times, durations) or plain strings.
//! The Apollo Federation scalars `_Any` and `_FieldSet` are not bound: they
//! only appear in federation internals that clients do not query.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::central::schema as central_schema;
use crate::series_state::schema as series_state_schema;
use crate::stats::schema as stats_schema;

cynic::impl_scalar!(jiff::Timestamp, central_schema::DateTime);
cynic::impl_scalar!(jiff::civil::Date, central_schema::Date);
cynic::impl_scalar!(rust_decimal::Decimal, central_schema::Decimal);
cynic::impl_scalar!(url::Url, central_schema::Url);
cynic::impl_scalar!(HexColor, central_schema::HexColor);

cynic::impl_scalar!(jiff::Timestamp, series_state_schema::DateTime);
cynic::impl_scalar!(jiff::Span, series_state_schema::Duration);
cynic::impl_scalar!(Version, series_state_schema::Version);

cynic::impl_scalar!(jiff::Span, stats_schema::Duration);

/// Hexadecimal RGB color as sent by GRID, for example `#ff0000`.
///
/// The value is kept exactly as received.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HexColor(String);

impl HexColor {
    /// Wraps a color string without validating it.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The color as received, for example `#ff0000`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HexColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Series State data-model version, formatted as `MAJOR.MINOR`.
///
/// The value is kept exactly as received; [`Version::major_minor`] parses it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Version(String);

impl Version {
    /// Wraps a version string without validating it.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The version as received, for example `3.17`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parses the version into `(major, minor)`, or `None` if it is not in
    /// `MAJOR.MINOR` format. The result orders like versions do.
    #[must_use]
    pub fn major_minor(&self) -> Option<(u32, u32)> {
        let (major, minor) = self.0.split_once('.')?;
        Some((major.parse().ok()?, minor.parse().ok()?))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip<T: Serialize + serde::de::DeserializeOwned>(json: &str) -> (T, String) {
        let value: T = serde_json::from_str(json).unwrap();
        let out = serde_json::to_string(&value).unwrap();
        (value, out)
    }

    #[test]
    fn date_time() {
        let (ts, out) = round_trip::<jiff::Timestamp>(r#""2024-05-01T12:34:56.789Z""#);
        assert_eq!(ts.as_millisecond(), 1_714_566_896_789);
        assert_eq!(out, r#""2024-05-01T12:34:56.789Z""#);
    }

    #[test]
    fn date() {
        let (date, out) = round_trip::<jiff::civil::Date>(r#""2024-05-01""#);
        assert_eq!(date, jiff::civil::date(2024, 5, 1));
        assert_eq!(out, r#""2024-05-01""#);
    }

    #[test]
    fn duration() {
        let (span, out) = round_trip::<jiff::Span>(r#""PT1M30.5S""#);
        let expected = jiff::Span::new().minutes(1).seconds(30).milliseconds(500);
        assert_eq!(span.fieldwise(), expected.fieldwise());
        let again: jiff::Span = serde_json::from_str(&out).unwrap();
        assert_eq!(again.fieldwise(), expected.fieldwise());
    }

    #[test]
    fn decimal() {
        let (dec, out) = round_trip::<rust_decimal::Decimal>(r#""1234.50""#);
        assert_eq!(dec, rust_decimal::Decimal::new(123_450, 2));
        assert_eq!(out, r#""1234.50""#);
    }

    #[test]
    fn url() {
        let (url, out) = round_trip::<url::Url>(r#""https://cdn.grid.gg/logo.png""#);
        assert_eq!(url.host_str(), Some("cdn.grid.gg"));
        assert_eq!(out, r#""https://cdn.grid.gg/logo.png""#);
    }

    #[test]
    fn hex_color() {
        let (color, out) = round_trip::<HexColor>(r##""#ff00aa""##);
        assert_eq!(color.as_str(), "#ff00aa");
        assert_eq!(out, r##""#ff00aa""##);
    }

    #[test]
    fn version() {
        let (version, out) = round_trip::<Version>(r#""3.17""#);
        assert_eq!(version.major_minor(), Some((3, 17)));
        assert!(version.major_minor() > Version::new("3.9").major_minor());
        assert_eq!(Version::new("abc").major_minor(), None);
        assert_eq!(out, r#""3.17""#);
    }
}
