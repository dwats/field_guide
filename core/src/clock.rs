//! Timestamps: RFC 3339 UTC with millisecond precision and a `Z` suffix, e.g.
//! `2026-11-07T19:02:11.123Z`. Fixed width, so string comparison is time comparison.

use chrono::{SecondsFormat, Utc};

pub(crate) fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
