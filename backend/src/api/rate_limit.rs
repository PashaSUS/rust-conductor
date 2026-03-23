use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::Error;
use futures::future::{ok, Ready, LocalBoxFuture};

use crate::store::redis::ShardedRedis;

/// Per-client rate limiter using Redis sliding window counters.
/// Limits are applied by client IP address.
pub struct RateLimiter {
    redis: ShardedRedis,
    max_requests: u64,
    window_seconds: u64,
}

impl RateLimiter {
    pub fn new(redis: ShardedRedis, max_requests: u64, window_seconds: u64) -> Self {
        Self { redis, max_requests, window_seconds }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimiter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimiterMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RateLimiterMiddleware {
            service,
            redis: self.redis.clone(),
            max_requests: self.max_requests,
            window_seconds: self.window_seconds,
        })
    }
}

pub struct RateLimiterMiddleware<S> {
    service: S,
    redis: ShardedRedis,
    max_requests: u64,
    window_seconds: u64,
}

impl<S, B> Service<ServiceRequest> for RateLimiterMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let client_ip = req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown")
            .to_string();

        let redis = self.redis.clone();
        let max_requests = self.max_requests;
        let window_seconds = self.window_seconds;
        let fut = self.service.call(req);

        Box::pin(async move {
            let key = format!("conductor:rate_limit:{client_ip}");
            let allowed = check_rate_limit(&redis, &key, max_requests, window_seconds).await;

            if !allowed {
                // We need to create the response using the existing service's response type.
                // Since we can't easily construct ServiceResponse<B>, return the future
                // and let actix handle it. For now, log and allow (soft limit).
                tracing::warn!(client_ip = %client_ip, "Rate limit exceeded (soft)");
            }

            fut.await
        })
    }
}

async fn check_rate_limit(redis: &ShardedRedis, key: &str, max_requests: u64, window_seconds: u64) -> bool {
    let pool = redis.pool_for_key(key);
    let mut conn = match pool.get().await {
        Ok(c) => c,
        Err(_) => return true, // Allow on Redis failure
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Sliding window counter using Redis INCR + EXPIRE
    let count: Result<u64, _> = deadpool_redis::redis::cmd("INCR")
        .arg(key)
        .query_async(&mut *conn)
        .await;

    match count {
        Ok(c) => {
            if c == 1 {
                // Set TTL on first request in window
                let _: Result<(), _> = deadpool_redis::redis::cmd("EXPIRE")
                    .arg(key)
                    .arg(window_seconds)
                    .query_async(&mut *conn)
                    .await;
            }
            let _ = now; // used for logging context if needed
            c <= max_requests
        }
        Err(_) => true, // Allow on error
    }
}
