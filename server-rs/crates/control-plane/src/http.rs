use async_nats::Client;
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde_json::{json, Value};
use sqlx::{query_scalar, PgPool};

#[derive(Clone)]
pub struct HttpState {
    pool: PgPool,
    nats: Client,
}

impl HttpState {
    pub fn new(pool: PgPool, nats: Client) -> Self {
        Self { pool, nats }
    }

    pub async fn readiness_check(&self) -> Result<(), String> {
        query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|error| format!("postgres not ready: {error}"))?;

        self.nats
            .flush()
            .await
            .map_err(|error| format!("nats not ready: {error}"))?;

        Ok(())
    }
}

pub fn router(state: HttpState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .with_state(state)
}

async fn healthz() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

async fn readyz(State(state): State<HttpState>) -> (StatusCode, Json<Value>) {
    match state.readiness_check().await {
        Ok(()) => (StatusCode::OK, Json(json!({ "status": "ready" }))),
        Err(error) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "not-ready", "error": error })),
        ),
    }
}
