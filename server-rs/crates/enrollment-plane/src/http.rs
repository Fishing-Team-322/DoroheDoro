use async_nats::Client;
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use common::health;
use serde_json::Value;
use sqlx::PgPool;

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
        health::check_postgres_and_nats(&self.pool, &self.nats).await
    }
}

pub fn router(state: HttpState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .with_state(state)
}

async fn healthz() -> (StatusCode, Json<Value>) {
    health::healthz_response()
}

async fn readyz(State(state): State<HttpState>) -> (StatusCode, Json<Value>) {
    health::readyz_response(state.readiness_check().await)
}
