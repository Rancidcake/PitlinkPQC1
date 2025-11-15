//! Metrics collection and storage for dashboard

use serde::{Serialize, Deserialize};
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};
use std::collections::VecDeque;

/// System-wide metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    
    // Network metrics
    pub network: NetworkMetrics,
    
    // AI decision metrics
    pub ai_decision: AiDecisionMetrics,
    
    // QUIC-FEC metrics
    pub quic_fec: QuicFecMetrics,
    
    // Compression metrics
    pub compression: CompressionMetrics,
    
    // System performance
    pub performance: PerformanceMetrics,
}

/// Network metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub rtt_ms: f32,
    pub jitter_ms: f32,
    pub loss_rate: f32,
    pub throughput_mbps: f32,
    pub wifi_signal: f32,
    pub fiveg_signal: Option<f32>,
    pub starlink_latency: Option<f32>,
    pub current_path: String,
    pub network_quality_score: f32,
}

/// AI decision metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDecisionMetrics {
    pub route: String,
    pub severity: String,
    pub should_send: bool,
    pub similarity_score: f32,
    pub optimization_hint: String,
    pub congestion_predicted: bool,
    pub wfq_weights: WfqWeights,
}

/// WFQ (Weighted Fair Queue) weights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WfqWeights {
    pub p0: u32,
    pub p1: u32,
    pub p2: u32,
}

/// QUIC-FEC metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuicFecMetrics {
    pub connected: bool,
    pub fec_enabled: bool,
    pub fec_config: FecConfigMetrics,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packets_recovered: u64,
    pub handover_count: u32,
    pub last_handover: Option<DateTime<Utc>>,
}

/// FEC configuration metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FecConfigMetrics {
    pub data_shards: usize,
    pub parity_shards: usize,
    pub redundancy_percent: f32,
}

/// Compression metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionMetrics {
    pub total_compressed: u64,
    pub total_uncompressed: u64,
    pub compression_ratio: f32,
    pub lz4_count: u64,
    pub zstd_count: u64,
    pub avg_compression_time_ms: f32,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub chunks_processed: u64,
    pub avg_processing_time_ms: f32,
    pub ai_inference_time_ms: f32,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub uptime_seconds: u64,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            network: NetworkMetrics {
                rtt_ms: 20.0, // Default reasonable values
                jitter_ms: 2.0,
                loss_rate: 0.001,
                throughput_mbps: 100.0,
                wifi_signal: -60.0,
                fiveg_signal: None,
                starlink_latency: None,
                current_path: "WiFi".to_string(),
                network_quality_score: 0.8, // Good quality
            },
            ai_decision: AiDecisionMetrics {
                route: "WiFi".to_string(),
                severity: "Low".to_string(),
                should_send: true,
                similarity_score: 0.5,
                optimization_hint: "SendFull".to_string(),
                congestion_predicted: false,
                wfq_weights: WfqWeights { p0: 50, p1: 30, p2: 20 },
            },
            quic_fec: QuicFecMetrics {
                connected: true, // Default to connected (dashboard is running)
                fec_enabled: true,
                fec_config: FecConfigMetrics {
                    data_shards: 8,
                    parity_shards: 3,
                    redundancy_percent: 37.5,
                },
                packets_sent: 0,
                packets_received: 0,
                packets_recovered: 0,
                handover_count: 0,
                last_handover: None,
            },
            compression: CompressionMetrics {
                total_compressed: 0,
                total_uncompressed: 0,
                compression_ratio: 0.0,
                lz4_count: 0,
                zstd_count: 0,
                avg_compression_time_ms: 0.0,
            },
            performance: PerformanceMetrics {
                chunks_processed: 0,
                avg_processing_time_ms: 0.0,
                ai_inference_time_ms: 0.0,
                total_bytes_sent: 0,
                total_bytes_received: 0,
                uptime_seconds: 0,
            },
        }
    }
}

/// Metrics collector that stores historical data
pub struct MetricsCollector {
    metrics: Arc<RwLock<SystemMetrics>>,
    history: Arc<RwLock<VecDeque<SystemMetrics>>>,
    max_history: usize,
    start_time: DateTime<Utc>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(max_history: usize) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(SystemMetrics::default())),
            history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history))),
            max_history,
            start_time: Utc::now(),
        }
    }

    /// Update current metrics
    pub fn update(&self, metrics: SystemMetrics) {
        let mut current = self.metrics.write();
        *current = metrics.clone();
        
        // Add to history
        let mut history = self.history.write();
        history.push_back(metrics);
        
        // Limit history size
        while history.len() > self.max_history {
            history.pop_front();
        }
    }

    /// Get current metrics
    pub fn get_current(&self) -> SystemMetrics {
        let mut metrics = self.metrics.read().clone();
        // Update uptime
        metrics.performance.uptime_seconds = 
            (Utc::now() - self.start_time).num_seconds() as u64;
        metrics
    }

    /// Get historical metrics
    pub fn get_history(&self, limit: Option<usize>) -> Vec<SystemMetrics> {
        let history = self.history.read();
        let limit = limit.unwrap_or(history.len());
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get metrics for a specific time range
    pub fn get_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<SystemMetrics> {
        let history = self.history.read();
        history.iter()
            .filter(|m| m.timestamp >= start && m.timestamp <= end)
            .cloned()
            .collect()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new(1000) // Keep last 1000 metrics
    }
}

