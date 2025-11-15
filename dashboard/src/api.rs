//! REST API endpoints for dashboard

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::metrics::MetricsCollector;

/// Create API router
pub fn create_api_router(collector: Arc<MetricsCollector>) -> Router {
    Router::new()
        .route("/metrics/current", get(get_current_metrics))
        .route("/metrics/history", get(get_history))
        .route("/health", get(health_check))
        .with_state(collector)
}

/// Get current metrics
async fn get_current_metrics(
    State(collector): State<Arc<MetricsCollector>>,
) -> Result<Json<Value>, StatusCode> {
    let metrics = collector.get_current();
    Ok(Json(json!(metrics)))
}

/// Get historical metrics
async fn get_history(
    State(collector): State<Arc<MetricsCollector>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok());
    
    let history = collector.get_history(limit);
    Ok(Json(json!(history)))
}

/// Health check endpoint
async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "pitlinkpqc-dashboard"
    }))
}

