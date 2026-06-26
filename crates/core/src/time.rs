//! Common time types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A UTC timestamp used throughout GetAGrip.
pub type Timestamp = DateTime<Utc>;

/// Returns the current UTC timestamp.
#[must_use]
pub fn now() -> Timestamp {
    Utc::now()
}

/// Formats a duration in a human-readable way.
#[must_use]
pub fn format_duration_ms(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms}ms")
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        format!("{mins}m {secs}s")
    }
}

/// A change timestamp for tracking when objects were last modified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ChangeTimestamp(pub Timestamp);

impl Default for ChangeTimestamp {
    fn default() -> Self {
        Self(now())
    }
}

impl ChangeTimestamp {
    /// Create a new change timestamp set to now.
    #[must_use]
    pub fn now() -> Self {
        Self(Utc::now())
    }
}
