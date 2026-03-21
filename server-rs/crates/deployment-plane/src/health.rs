use async_nats::Client;
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use common::health;
use serde_json::Value;
use sqlx::PgPool;

use crate::executor::DynDeploymentExecutor;

#[derive(Clone)]
pub struct HealthState {
    pool: PgPool,
    nats: Client,
    executor: DynDeploymentExecutor,
}

impl HealthState {
    pub fn new(pool: PgPool, nats: Client, executor: DynDeploymentExecutor) -> Self {
        Self {
            pool,
            nats,
            executor,
        }
    }

    pub async fn readiness_check(&self) -> Result<(), String> {
        health::check_postgres_and_nats(&self.pool, &self.nats).await?;

        self.executor
            .readiness_check()
            .await
            .map_err(|error| format!("executor not ready: {error}"))?;

        Ok(())
    }
}

pub fn router(state: HealthState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .with_state(state)
}

async fn healthz() -> (StatusCode, Json<Value>) {
    health::healthz_response()
}

async fn readyz(State(state): State<HealthState>) -> (StatusCode, Json<Value>) {
    health::readyz_response(state.readiness_check().await)
}
