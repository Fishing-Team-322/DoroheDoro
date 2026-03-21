use async_nats::Client;
use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};
use sqlx::{query_scalar, PgPool};

pub async fn check_postgres_and_nats(pool: &PgPool, nats: &Client) -> Result<(), String> {
    query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
        .map_err(|error| format!("postgres not ready: {error}"))?;

    nats.flush()
        .await
        .map_err(|error| format!("nats not ready: {error}"))?;

    Ok(())
}

pub fn healthz_response() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

pub fn readyz_response(result: Result<(), String>) -> (StatusCode, Json<Value>) {
    match result {
        Ok(()) => (StatusCode::OK, Json(json!({ "status": "ready" }))),
        Err(error) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "not-ready", "error": error })),
        ),
    }
}
