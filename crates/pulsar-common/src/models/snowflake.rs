use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicI64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Snowflake(pub i64);

// 2026-01-01T00:00:00Z
const EPOCH: i64 = 1_767_225_600_000;
const WORKER_ID_BITS: i64 = 10;
const SEQUENCE_BITS: i64 = 12;
const WORKER_ID_SHIFT: i64 = SEQUENCE_BITS;
const TIMESTAMP_SHIFT: i64 = WORKER_ID_BITS + SEQUENCE_BITS;
const SEQUENCE_MASK: i64 = (1 << SEQUENCE_BITS) - 1;
const MAX_WORKER_ID: i64 = (1 << WORKER_ID_BITS) - 1;

static LAST_STATE: AtomicI64 = AtomicI64::new(0);
static WORKER_ID: OnceLock<i64> = OnceLock::new();

fn worker_id() -> i64 {
    *WORKER_ID.get_or_init(|| {
        std::env::var("WORKER_ID")
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0)
            .clamp(0, MAX_WORKER_ID)
    })
}

fn next_state(last: i64, now_ms: i64, wid: i64) -> i64 {
    let last_ts = last >> TIMESTAMP_SHIFT;
    let last_seq = last & SEQUENCE_MASK;
    if now_ms > last_ts {
        (now_ms << TIMESTAMP_SHIFT) | (wid << WORKER_ID_SHIFT)
    } else {
        let next_seq = (last_seq + 1) & SEQUENCE_MASK;
        let ts = if next_seq == 0 { last_ts + 1 } else { last_ts };
        (ts << TIMESTAMP_SHIFT) | (wid << WORKER_ID_SHIFT) | next_seq
    }
}

impl Snowflake {
    pub fn generate() -> Self {
        let wid = worker_id();
        let now_ms = chrono::Utc::now().timestamp_millis() - EPOCH;
        let mut new_state = 0i64;
        LAST_STATE
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |last| {
                new_state = next_state(last, now_ms, wid);
                Some(new_state)
            })
            .expect("fetch_update closure always returns Some");
        Self(new_state)
    }
}

impl Display for Snowflake {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
