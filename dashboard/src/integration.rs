//! Integration helpers for connecting dashboard with system components

use crate::metrics::{SystemMetrics, MetricsCollector, NetworkMetrics, AiDecisionMetrics, 
                     QuicFecMetrics, CompressionMetrics, PerformanceMetrics, WfqWeights, FecConfigMetrics};
use trackshift::telemetry_ai::{AiDecision, NetworkMetricsInput, RouteDecision, Severity, OptimizationHint};
use quic_fec::NetworkPath;
use std::sync::Arc;
use chrono::Utc;

/// Convert AI decision to dashboard metrics
pub fn ai_decision_to_metrics(decision: &AiDecision) -> AiDecisionMetrics {
    AiDecisionMetrics {
        route: format!("{:?}", decision.route),
        severity: format!("{:?}", decision.severity),
        should_send: decision.should_send,
        similarity_score: decision.similarity_score,
        optimization_hint: format!("{:?}", decision.optimization_hint),
        congestion_predicted: decision.congestion_predicted,
        wfq_weights: WfqWeights {
            p0: decision.wfq_p0_weight,
            p1: decision.wfq_p1_weight,
            p2: decision.wfq_p2_weight,
        },
    }
}

/// Convert network metrics input to dashboard metrics
pub fn network_input_to_metrics(
    input: &NetworkMetricsInput,
    current_path: Option<NetworkPath>,
    quality_score: f32,
) -> NetworkMetrics {
    NetworkMetrics {
        rtt_ms: input.rtt_ms,
        jitter_ms: input.jitter_ms,
        loss_rate: input.loss_rate,
        throughput_mbps: input.throughput_mbps,
        wifi_signal: input.wifi_signal,
        fiveg_signal: None, // Would need to add to NetworkMetricsInput
        starlink_latency: Some(input.starlink_latency),
        current_path: current_path
            .map(|p| format!("{:?}", p))
            .unwrap_or_else(|| "Unknown".to_string()),
        network_quality_score: quality_score,
    }
}

/// Helper to update dashboard metrics from system state
pub fn update_dashboard_metrics(
    collector: Arc<MetricsCollector>,
    network_input: &NetworkMetricsInput,
    ai_decision: &AiDecision,
    quic_connected: bool,
    fec_enabled: bool,
    fec_data_shards: usize,
    fec_parity_shards: usize,
    packets_sent: u64,
    packets_received: u64,
    packets_recovered: u64,
    handover_count: u32,
    compression_stats: Option<CompressionStats>,
    performance_stats: Option<PerformanceStats>,
) {
    let redundancy_percent = if fec_data_shards > 0 {
        (fec_parity_shards as f32 / (fec_data_shards + fec_parity_shards) as f32) * 100.0
    } else {
        0.0
    };

    let metrics = SystemMetrics {
        timestamp: Utc::now(),
        network: network_input_to_metrics(
            network_input,
            None, // Would get from QUIC-FEC connection
            ai_decision.network_quality.score,
        ),
        ai_decision: ai_decision_to_metrics(ai_decision),
        quic_fec: QuicFecMetrics {
            connected: quic_connected,
            fec_enabled,
            fec_config: FecConfigMetrics {
                data_shards: fec_data_shards,
                parity_shards: fec_parity_shards,
                redundancy_percent,
            },
            packets_sent,
            packets_received,
            packets_recovered,
            handover_count,
            last_handover: None, // Would track from handover events
        },
        compression: compression_stats
            .map(|s| CompressionMetrics {
                total_compressed: s.total_compressed,
                total_uncompressed: s.total_uncompressed,
                compression_ratio: if s.total_uncompressed > 0 {
                    s.total_compressed as f32 / s.total_uncompressed as f32
                } else {
                    0.0
                },
                lz4_count: s.lz4_count,
                zstd_count: s.zstd_count,
                avg_compression_time_ms: s.avg_compression_time_ms,
            })
            .unwrap_or_default(),
        performance: performance_stats
            .map(|s| PerformanceMetrics {
                chunks_processed: s.chunks_processed,
                avg_processing_time_ms: s.avg_processing_time_ms,
                ai_inference_time_ms: s.ai_inference_time_ms,
                total_bytes_sent: s.total_bytes_sent,
                total_bytes_received: s.total_bytes_received,
                uptime_seconds: s.uptime_seconds,
            })
            .unwrap_or_default(),
    };

    collector.update(metrics);
}

/// Compression statistics
pub struct CompressionStats {
    pub total_compressed: u64,
    pub total_uncompressed: u64,
    pub lz4_count: u64,
    pub zstd_count: u64,
    pub avg_compression_time_ms: f32,
}

/// Performance statistics
pub struct PerformanceStats {
    pub chunks_processed: u64,
    pub avg_processing_time_ms: f32,
    pub ai_inference_time_ms: f32,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub uptime_seconds: u64,
}

impl Default for CompressionMetrics {
    fn default() -> Self {
        Self {
            total_compressed: 0,
            total_uncompressed: 0,
            compression_ratio: 0.0,
            lz4_count: 0,
            zstd_count: 0,
            avg_compression_time_ms: 0.0,
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            chunks_processed: 0,
            avg_processing_time_ms: 0.0,
            ai_inference_time_ms: 0.0,
            total_bytes_sent: 0,
            total_bytes_received: 0,
            uptime_seconds: 0,
        }
    }
}

