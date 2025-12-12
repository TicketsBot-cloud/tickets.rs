use crate::http::response::Response;
use crate::http::Server;
use axum::extract::Extension;
use axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN;
use axum::http::{HeaderMap, HeaderValue};
use axum::response::Json;
use cache::Cache;
use hyper::http::StatusCode;
use std::env;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub async fn total_handler<T: Cache>(
    server: Extension<Arc<Server<T>>>,
) -> (StatusCode, HeaderMap, Json<Response>) {
    let count = server.0.count.load(Ordering::Relaxed);

    let origin = env::var("CORS_ORIGIN")
        .unwrap_or_else(|_| "https://tickets.bot".to_string());

    let mut headers = HeaderMap::new();
    headers.insert(
        ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_str(&origin).expect("Invalid CORS_ORIGIN"),
    );

    let body = Response::success(count);

    (StatusCode::OK, headers, Json(body))
}
