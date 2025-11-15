//! Dashboard for PitlinkPQC System
//! 
//! Provides a web-based dashboard for monitoring:
//! - Telemetry AI decisions
//! - Network metrics and handover events
//! - QUIC-FEC connection status
//! - Compression statistics
//! - System performance

pub mod api;
pub mod metrics;
pub mod server;
pub mod integration;

pub use server::DashboardServer;
pub use metrics::{SystemMetrics, MetricsCollector};
pub use integration::{update_dashboard_metrics, CompressionStats, PerformanceStats};

