//! Dashboard web server

use axum::{
    Router,
    routing::get,
    response::Html,
};
use tower_http::cors::CorsLayer;
use std::sync::Arc;
use anyhow::Result;
use crate::api::create_api_router;
use crate::metrics::MetricsCollector;

/// Dashboard server
pub struct DashboardServer {
    collector: Arc<MetricsCollector>,
    port: u16,
}

impl DashboardServer {
    /// Create a new dashboard server
    pub fn new(port: u16) -> Self {
        Self {
            collector: Arc::new(MetricsCollector::new(1000)),
            port,
        }
    }

    /// Get metrics collector reference
    pub fn collector(&self) -> Arc<MetricsCollector> {
        self.collector.clone()
    }

    /// Start the dashboard server
    pub async fn start(&self) -> Result<()> {
        let app = self.create_app();
        
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        
        println!("🚀 Dashboard server starting on http://{}", addr);
        println!("📊 Open http://localhost:{} in your browser", self.port);
        
        axum::serve(listener, app).await?;
        
        Ok(())
    }

    /// Create the Axum application
    fn create_app(&self) -> Router {
        // API routes
        let api_router = create_api_router(self.collector.clone());
        
        // Main router
        Router::new()
            .route("/", get(index_page))
            .nest("/api", api_router)
            .layer(CorsLayer::permissive())
    }
}

/// Index page handler - serve embedded HTML
async fn index_page() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

