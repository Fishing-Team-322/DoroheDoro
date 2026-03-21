use std::sync::Arc;

use axum::{extract::State, routing::get, Json, Router};
use serde_json::json;

use crate::service::QueryAlertService;

#[derive(Clone)]
pub struct HttpState {
    pub service: Arc<QueryAlertService>,
}

impl HttpState {
    pub fn new(service: Arc<QueryAlertService>) -> Self {
        Self { service }
    }
}

pub fn router(state: HttpState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/status/anomaly", get(anomaly_status))
        .with_state(state)
}

async fn healthz() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

async fn readyz(State(state): State<HttpState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": if state.service.is_ready() { "ready" } else { "starting" }
    }))
}

async fn anomaly_status(State(state): State<HttpState>) -> Json<serde_json::Value> {
    let snapshot = state.service.detection_snapshot();
    Json(json!({
        "mode": snapshot.mode.as_str(),
        "safe_mode": snapshot.safe_mode,
        "signals_total": snapshot.signals_total,
        "anomalies_total": snapshot.anomalies_total,
        "resolved_total": snapshot.resolved_total,
        "rejected_total": snapshot.rejected_total,
    }))
}
