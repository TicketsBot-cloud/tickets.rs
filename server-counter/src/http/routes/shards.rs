use crate::http::Server;
use axum::extract::Extension;
use axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN;
use axum::http::{HeaderMap, HeaderValue};
use axum::response::Json;
use cache::Cache;
use hyper::http::StatusCode;
use std::env;
use std::sync::Arc;

use crate::http::server::ShardsSnapshot;

pub async fn shards_handler<T: Cache>(
    server: Extension<Arc<Server<T>>>,
) -> (StatusCode, HeaderMap, Json<ShardsSnapshot>) {
    let origin = env::var("CORS_ORIGIN").unwrap_or_else(|_| "https://tickets.bot".to_string());

    let mut headers = HeaderMap::new();
    headers.insert(
        ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_str(&origin).expect("Invalid CORS_ORIGIN"),
    );

    let snapshot = server
        .0
        .shards_snapshot
        .read()
        .map(|guard| guard.clone())
        .unwrap_or_default();

    (StatusCode::OK, headers, Json(snapshot))
}
