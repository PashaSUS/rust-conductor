use std::sync::OnceLock;
use std::time::Duration;

use actix_web::error::{ErrorBadGateway, ErrorBadRequest, ErrorForbidden};
use actix_web::http::StatusCode;
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::{HttpRequest, HttpResponse, web};
use serde::Deserialize;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

#[derive(Deserialize)]
struct ProxyQuery {
    target: String,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/proxy").route(web::to(proxy_request)));
}

async fn proxy_request(
    req: HttpRequest,
    query: web::Query<ProxyQuery>,
    body: web::Bytes,
) -> Result<HttpResponse, actix_web::Error> {
    let target = reqwest::Url::parse(&query.target)
        .map_err(|_| ErrorBadRequest("invalid proxy target URL"))?;

    if !matches!(target.scheme(), "http" | "https") {
        return Err(ErrorBadRequest("proxy target must use http or https"));
    }

    if !is_conductor_path(target.path()) {
        return Err(ErrorBadRequest(
            "proxy target must be a Conductor REST path",
        ));
    }

    if !is_allowed_host(&target) {
        return Err(ErrorForbidden("proxy target host is not allowed"));
    }

    let method = reqwest::Method::from_bytes(req.method().as_str().as_bytes())
        .map_err(|_| ErrorBadRequest("invalid HTTP method"))?;
    let client = CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .expect("valid proxy client")
    });

    let mut outbound = client
        .request(method, target)
        .header("accept-encoding", "identity");
    for (name, value) in req.headers() {
        if should_skip_request_header(name.as_str()) {
            continue;
        }
        outbound = outbound.header(name.as_str(), value.as_bytes());
    }

    if !body.is_empty() {
        outbound = outbound.body(body.to_vec());
    }

    let upstream = outbound
        .send()
        .await
        .map_err(|e| ErrorBadGateway(format!("proxy request failed: {e}")))?;
    let status = StatusCode::from_u16(upstream.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut response = HttpResponse::build(status);

    for (name, value) in upstream.headers() {
        if should_skip_response_header(name.as_str()) {
            continue;
        }
        if let (Ok(header_name), Ok(header_value)) = (
            HeaderName::from_bytes(name.as_str().as_bytes()),
            HeaderValue::from_bytes(value.as_bytes()),
        ) {
            response.append_header((header_name, header_value));
        }
    }

    let bytes = upstream
        .bytes()
        .await
        .map_err(|e| ErrorBadGateway(format!("proxy response failed: {e}")))?;
    Ok(response.body(bytes))
}

fn should_skip_request_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "host"
            | "connection"
            | "accept-encoding"
            | "content-length"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "origin"
            | "referer"
            | "cookie"
    )
}

fn is_conductor_path(path: &str) -> bool {
    let path = path.trim_end_matches('/');
    path == "/api"
        || path.ends_with("/api")
        || path.contains("/api/")
        || path == "/health"
        || path.ends_with("/health")
}

fn is_allowed_host(target: &reqwest::Url) -> bool {
    let Ok(allowed) = std::env::var("CONDUCTOR_PROXY_ALLOWED_HOSTS") else {
        return true;
    };

    let Some(host) = target.host_str() else {
        return false;
    };
    let host_with_port = target
        .port_or_known_default()
        .map(|port| format!("{host}:{port}"));

    allowed.split(',').map(str::trim).any(|entry| {
        !entry.is_empty()
            && (entry.eq_ignore_ascii_case(host)
                || host_with_port
                    .as_deref()
                    .is_some_and(|host_port| entry.eq_ignore_ascii_case(host_port)))
    })
}

fn should_skip_response_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "content-length"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}
