use std::{collections::HashMap, sync::Arc, time::{Duration, Instant}};
use tokio::sync::Mutex;

const MAX_EVENTS: u32 = 20;
const WINDOW: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct GatewayRateLimiter(Arc<Mutex<HashMap<String, (Instant, u32)>>>);

impl GatewayRateLimiter {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }

    pub async fn check(&self, user_id: &str) -> bool {
        let mut map = self.0.lock().await;
        let now = Instant::now();
        map.retain(|_, (ts, _)| now.duration_since(*ts) < WINDOW);
        let entry = map.entry(user_id.to_string()).or_insert((now, 0));
        entry.1 += 1;
        entry.1 <= MAX_EVENTS
    }
}
