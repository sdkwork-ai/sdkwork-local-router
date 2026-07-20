use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use serde_json::json;
use std::time::Instant;

use crate::router::AppState;

const MAX_ENTRIES: usize = 100_000;

struct RateLimitEntry {
    count: u64,
    window_start: Instant,
}

pub struct RateLimiter {
    entries: DashMap<u64, RateLimitEntry>,
    max_requests: u64,
    window_secs: u64,
}

impl RateLimiter {
    pub fn new(max_requests: u64, window_secs: u64) -> Self {
        Self {
            entries: DashMap::new(),
            max_requests,
            window_secs,
        }
    }

    pub fn check(&self, key: u64) -> bool {
        if self.entries.len() >= MAX_ENTRIES {
            self.cleanup();
        }

        let now = Instant::now();

        if let Some(mut entry) = self.entries.get_mut(&key) {
            if now.duration_since(entry.window_start).as_secs() >= self.window_secs {
                entry.count = 0;
                entry.window_start = now;
            }
            if entry.count >= self.max_requests {
                return false;
            }
            entry.count += 1;
            return true;
        }

        self.entries.insert(
            key,
            RateLimitEntry {
                count: 1,
                window_start: now,
            },
        );
        true
    }

    pub fn cleanup(&self) {
        let now = Instant::now();
        self.entries.retain(|_, entry| {
            now.duration_since(entry.window_start).as_secs() < self.window_secs * 2
        });
    }
}

pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    if !state.config.rate_limit.enabled {
        return next.run(req).await;
    }

    let client_key = extract_client_key_hash(&req);

    if state.rate_limiter.check(client_key) {
        next.run(req).await
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            axum::Json(json!({
                "error": {
                    "message": "rate limit exceeded",
                    "type": "rate_limit_error",
                    "code": 429
                }
            })),
        )
            .into_response()
    }
}

fn extract_client_key_hash(req: &Request) -> u64 {
    if let Some(auth) = req.headers().get(axum::http::header::AUTHORIZATION) {
        if let Ok(val) = auth.to_str() {
            return fnv1a_64(val.as_bytes());
        }
    }

    if let Some(key) = req.headers().get("x-api-key") {
        if let Ok(val) = key.to_str() {
            return fnv1a_64(val.as_bytes());
        }
    }

    if let Some(xff) = req.headers().get("x-forwarded-for") {
        if let Ok(val) = xff.to_str() {
            let client_ip = val.split(',').next().unwrap_or("unknown").trim();
            return fnv1a_64(client_ip.as_bytes());
        }
    }

    if let Some(rip) = req.headers().get("x-real-ip") {
        if let Ok(val) = rip.to_str() {
            return fnv1a_64(val.as_bytes());
        }
    }

    fnv1a_64(b"ip:unknown")
}

fn fnv1a_64(data: &[u8]) -> u64 {
    const FNV_PRIME: u64 = 0x100000001b3;
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
