use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicI64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Snowflake(pub i64);

static LAST_ID: AtomicI64 = AtomicI64::new(0);

impl Snowflake {
    pub fn generate() -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        let prev = LAST_ID
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |last| {
                Some(last.max(now - 1) + 1)
            })
            .unwrap_or(now - 1);
        Self(prev.max(now - 1) + 1)
    }
}

impl Display for Snowflake {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
