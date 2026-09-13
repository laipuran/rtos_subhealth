pub mod http;
pub mod ws;

use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::Value;

use crate::error::{ApiError, ErrorCode};

/// Extract an incoming `X-Trace-Id`, or generate a 16-hex one (RFC-005 §5.3.4).
pub fn trace_id(headers: &HeaderMap) -> String {
    headers
        .get("x-trace-id")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            let s = uuid::Uuid::new_v4().simple().to_string();
            s[..16].to_string()
        })
}

fn with_trace(mut body: Value, tid: &str) -> Value {
    if let Some(obj) = body.as_object_mut() {
        obj.insert("trace_id".into(), Value::String(tid.to_string()));
    }
    body
}

/// A JSON response carrying the RFC-005 `trace_id` in both header and body.
pub fn json_response(status: u16, body: Value, tid: &str) -> Response {
    let body = with_trace(body, tid);
    let mut resp = (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(body),
    )
        .into_response();
    if let Ok(v) = HeaderValue::from_str(tid) {
        resp.headers_mut().insert("x-trace-id", v);
    }
    resp
}

/// An RFC-005 error response: `{"error":{"code","message","details"?},"trace_id"}`.
pub fn error_response(err: &ApiError, tid: &str) -> Response {
    let body = with_trace(err.to_body(), tid);
    let mut resp = (
        StatusCode::from_u16(err.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(body),
    )
        .into_response();
    if let Ok(v) = HeaderValue::from_str(tid) {
        resp.headers_mut().insert("x-trace-id", v);
    }
    resp
}

/// Return `Some(401)` when auth is enabled and the `X-API-Key` header is
/// missing or wrong; `None` when the request may proceed.
pub fn check_auth(token: &str, headers: &HeaderMap, tid: &str) -> Option<Response> {
    if token.is_empty() {
        return None;
    }
    let provided = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if provided != token {
        return Some(error_response(
            &ApiError::new(ErrorCode::Unauthorized, "invalid or missing X-API-Key"),
            tid,
        ));
    }
    None
}

/// Parse `offset`/`limit` query parameters with RFC-005 defaults and clamps.
/// Returns `None` when a provided value is not an integer.
pub fn parse_pagination(query: &std::collections::HashMap<String, String>) -> Option<(i64, i64)> {
    let offset = match query.get("offset") {
        Some(v) => {
            let parsed: i64 = v.parse().ok()?;
            parsed.max(0)
        }
        None => 0,
    };
    let limit = match query.get("limit") {
        Some(v) => {
            let parsed: i64 = v.parse().ok()?;
            parsed.clamp(1, 200)
        }
        None => 50,
    };
    Some((offset, limit))
}

/// Header name helper used by the map ETag logic.
pub fn etag_header(value: &str) -> Option<(header::HeaderName, HeaderValue)> {
    HeaderValue::from_str(value).ok().map(|v| (header::ETAG, v))
}
