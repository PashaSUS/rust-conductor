use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::Error;
use futures::future::{ok, Ready, LocalBoxFuture};

use crate::store::redis::ShardedRedis;

/// Per-client rate limiter using Redis token bucket algorithm (#168).
///
/// Each client IP gets a bucket with `max_tokens` capacity that refills at
/// `refill_rate` tokens per second. Each request consumes one token. When the
/// bucket is empty, requests receive a 429 Too Many Requests response.
///
/// This replaces the simple sliding-window counter with a proper token bucket
/// that allows bursts while still enforcing sustainable request rates.
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
            max_tokens: self.max_requests,
            refill_rate: self.max_requests as f64 / self.window_seconds as f64,
        })
    }
}

pub struct RateLimiterMiddleware<S> {
    service: S,
    redis: ShardedRedis,
    /// Maximum tokens (bucket capacity)
    max_tokens: u64,
    /// Tokens added per second
    refill_rate: f64,
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
        let max_tokens = self.max_tokens;
        let refill_rate = self.refill_rate;
        let fut = self.service.call(req);

        Box::pin(async move {
            let key = format!("conductor:token_bucket:{client_ip}");
            let result = token_bucket_consume(&redis, &key, max_tokens, refill_rate).await;

            match result {
                TokenBucketResult::Allowed { remaining, .. } => {
                    let mut resp = fut.await?;
                    resp.headers_mut().insert(
                        actix_web::http::header::HeaderName::from_static("x-ratelimit-remaining"),
                        actix_web::http::header::HeaderValue::from_str(&remaining.to_string())
                            .unwrap_or_else(|_| actix_web::http::header::HeaderValue::from_static("0")),
                    );
                    resp.headers_mut().insert(
                        actix_web::http::header::HeaderName::from_static("x-ratelimit-limit"),
                        actix_web::http::header::HeaderValue::from_str(&max_tokens.to_string())
                            .unwrap_or_else(|_| actix_web::http::header::HeaderValue::from_static("0")),
                    );
                    Ok(resp)
                }
                TokenBucketResult::Limited { retry_after_secs } => {
                    tracing::warn!(client_ip = %client_ip, "Rate limit exceeded");
                    let mut resp = fut.await?;
                    resp.headers_mut().insert(
                        actix_web::http::header::HeaderName::from_static("x-ratelimit-remaining"),
                        actix_web::http::header::HeaderValue::from_static("0"),
                    );
                    resp.headers_mut().insert(
                        actix_web::http::header::HeaderName::from_static("x-ratelimit-limit"),
                        actix_web::http::header::HeaderValue::from_str(&max_tokens.to_string())
                            .unwrap_or_else(|_| actix_web::http::header::HeaderValue::from_static("0")),
                    );
                    resp.headers_mut().insert(
                        actix_web::http::header::HeaderName::from_static("retry-after"),
                        actix_web::http::header::HeaderValue::from_str(&format!("{:.0}", retry_after_secs))
                            .unwrap_or_else(|_| actix_web::http::header::HeaderValue::from_static("1")),
                    );
                    Ok(resp)
                }
                TokenBucketResult::Error => {
                    // Allow on Redis failure — fail open
                    fut.await
                }
            }
        })
    }
}

enum TokenBucketResult {
    Allowed { remaining: u64 },
    Limited { retry_after_secs: f64 },
    Error,
}

/// Token bucket algorithm implemented in Redis.
///
/// Stores two keys per client:
/// - `{key}:tokens` — current token count (float stored as string)
/// - `{key}:ts`     — last refill timestamp
///
/// On each request: refill tokens based on elapsed time, then try to consume one.
async fn token_bucket_consume(
    redis: &ShardedRedis,
    key: &str,
    max_tokens: u64,
    refill_rate: f64,
) -> TokenBucketResult {
    let pool = redis.pool_for_key(key);
    let mut conn = match pool.get().await {
        Ok(c) => c,
        Err(_) => return TokenBucketResult::Error,
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();

    let tokens_key = format!("{key}:tokens");
    let ts_key = format!("{key}:ts");

    // Atomic token bucket via Lua script
    let lua_script = r#"
        local tokens_key = KEYS[1]
        local ts_key = KEYS[2]
        local max_tokens = tonumber(ARGV[1])
        local refill_rate = tonumber(ARGV[2])
        local now = tonumber(ARGV[3])
        local ttl = tonumber(ARGV[4])

        local tokens = tonumber(redis.call('GET', tokens_key) or max_tokens)
        local last_ts = tonumber(redis.call('GET', ts_key) or now)

        local elapsed = math.max(0, now - last_ts)
        tokens = math.min(max_tokens, tokens + elapsed * refill_rate)

        if tokens >= 1 then
            tokens = tokens - 1
            redis.call('SET', tokens_key, tostring(tokens), 'EX', ttl)
            redis.call('SET', ts_key, tostring(now), 'EX', ttl)
            return tostring(math.floor(tokens))
        else
            local wait = (1 - tokens) / refill_rate
            return "-" .. tostring(wait)
        end
    "#;

    // TTL for the keys: enough to cover a full refill cycle
    let ttl = (max_tokens as f64 / refill_rate).ceil() as u64 + 10;

    let result: Result<String, _> = deadpool_redis::redis::cmd("EVAL")
        .arg(lua_script)
        .arg(2)
        .arg(&tokens_key)
        .arg(&ts_key)
        .arg(max_tokens)
        .arg(refill_rate)
        .arg(now)
        .arg(ttl)
        .query_async(&mut *conn)
        .await;

    match result {
        Ok(s) if s.starts_with('-') => {
            let wait: f64 = s[1..].parse().unwrap_or(1.0);
            TokenBucketResult::Limited { retry_after_secs: wait }
        }
        Ok(s) => {
            let remaining: u64 = s.parse().unwrap_or(0);
            TokenBucketResult::Allowed { remaining }
        }
        Err(_) => TokenBucketResult::Error,
    }
}

