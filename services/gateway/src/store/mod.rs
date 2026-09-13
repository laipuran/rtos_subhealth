pub mod diagnosis_store;
pub mod task_store;

use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds since the Unix epoch as a float, matching the legacy Python
/// `time.time()` timestamps stored in SQLite.
pub fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}
