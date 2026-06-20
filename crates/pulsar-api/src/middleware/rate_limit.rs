use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};
use axum::{
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use tokio::sync::Mutex;

const MAX_REQUESTS: u32 = 10;
const WINDOW: Duration = Duration::from_secs(60);

#[derive(Clone, Default)]
pub struct AuthRateLimiter(Arc<Mutex<HashMap<IpAddr, (Instant, u32)>>>);

impl AuthRateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    async fn check(&self, ip: IpAddr) -> bool {
        let mut map = self.0.lock().await;
        let now = Instant::now();
        map.retain(|_, (ts, _)| now.duration_since(*ts) < WINDOW);
        let entry = map.entry(ip).or_insert((now, 0));
        entry.1 += 1;
        entry.1 <= MAX_REQUESTS
    }
}

pub async fn auth_rate_limit_fn(
    limiter: AuthRateLimiter,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]));

    if !limiter.check(ip).await {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    Ok(next.run(request).await)
}
