//! Timestamps: RFC 3339 UTC with millisecond precision and a `Z` suffix, e.g.
//! `2026-11-07T19:02:11.123Z`. Fixed width, so string comparison is time comparison.

use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{Duration, SecondsFormat, Utc};

/// Added to every timestamp. Only ever grows, so order is preserved across threads.
static OFFSET_MS: AtomicI64 = AtomicI64::new(0);

pub(crate) fn now() -> String {
    let offset = Duration::milliseconds(OFFSET_MS.load(Ordering::Relaxed));
    (Utc::now() + offset).to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Moves every later timestamp forward by `ms`, so a test can make "later" happen without
/// sleeping. Process-wide and cumulative.
#[doc(hidden)]
pub fn advance_for_tests(ms: u32) {
    OFFSET_MS.fetch_add(i64::from(ms), Ordering::Relaxed);
}
