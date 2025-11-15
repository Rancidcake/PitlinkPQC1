//! Telemetry and metrics emission for multipath QUIC
//!
//! Produces structured metrics in JSON format for dashboard integration

use serde::{Serialize, Deserialize};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::Instant;
use std::collections::HashMap;

use crate::handover::NetworkPath;
use crate::scheduler::{PacketPriority, SchedulerStats};
use crate::fec_enhanced::FecStats;
use crate::receiver::ReceiverStats;
use crate::handover_enhanced::HandoverEvent;

/// Comprehensive multipath metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipathMetrics {
    pub timestamp: String,
    
    // Path metrics
    pub paths: HashMap<String, PathMetricsData>,
    
    // Active path utilization
    pub path_utilization: HashMap<String, f32>,
    
    // Handover events
    pub recent_handovers: Vec<HandoverEventData>,
    
    // FEC statistics
    pub fec_stats: FecStatsData,
    
    // QUIC checksum failures
    pub checksum_failures: u64,
    
    // Per-priority queue depths
    pub queue_depths: HashMap<String, usize>,
    
    // Scheduler statistics
    pub scheduler_stats: SchedulerStatsData,
    
    // Receiver statistics
    pub receiver_stats: ReceiverStatsData,
}

/// Path metrics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathMetricsData {
    pub rtt_ms: f32,
    pub loss_rate: f32,
    pub throughput_mbps: f32,
    pub congestion_window: u32,
    pub queue_depth: usize,
    pub active_streams: usize,
    pub is_active: bool,
}

/// Handover event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoverEventData {
    pub timestamp: String,
    pub from_path: String,
    pub to_path: String,
    pub reason: String,
    pub priority_streams_moved: usize,
    pub bulk_streams_moved: usize,
}

/// FEC statistics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FecStatsData {
    pub total_blocks: usize,
    pub repaired_blocks: usize,
    pub failed_blocks: usize,
    pub repair_rate: f32,
    pub parity_generation_rate: f32,
}

/// Scheduler statistics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStatsData {
    pub packets_scheduled: u64,
    pub packets_per_priority: HashMap<String, u64>,
    pub packets_per_path: HashMap<String, u64>,
    pub handover_events: u32,
    pub path_failures: u32,
    pub total_bytes_scheduled: u64,
}

/// Receiver statistics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiverStatsData {
    pub packets_received: u64,
    pub packets_reassembled: u64,
    pub checksum_failures: u64,
    pub fec_recovered: u64,
    pub decompression_failures: u64,
    pub total_bytes_received: u64,
    pub total_bytes_decompressed: u64,
}

/// Metrics emitter for dashboard
pub struct MetricsEmitter {
    metrics: Arc<RwLock<MultipathMetrics>>,
    handover_history: Arc<RwLock<Vec<HandoverEvent>>>,
    max_handover_history: usize,
}

impl MetricsEmitter {
    /// Create new metrics emitter
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(MultipathMetrics {
                timestamp: chrono::Utc::now().to_rfc3339(),
                paths: HashMap::new(),
                path_utilization: HashMap::new(),
                recent_handovers: Vec::new(),
                fec_stats: FecStatsData {
                    total_blocks: 0,
                    repaired_blocks: 0,
                    failed_blocks: 0,
                    repair_rate: 0.0,
                    parity_generation_rate: 0.0,
                },
                checksum_failures: 0,
                queue_depths: HashMap::new(),
                scheduler_stats: SchedulerStatsData {
                    packets_scheduled: 0,
                    packets_per_priority: HashMap::new(),
                    packets_per_path: HashMap::new(),
                    handover_events: 0,
                    path_failures: 0,
                    total_bytes_scheduled: 0,
                },
                receiver_stats: ReceiverStatsData {
                    packets_received: 0,
                    packets_reassembled: 0,
                    checksum_failures: 0,
                    fec_recovered: 0,
                    decompression_failures: 0,
                    total_bytes_received: 0,
                    total_bytes_decompressed: 0,
                },
            })),
            handover_history: Arc::new(RwLock::new(Vec::new())),
            max_handover_history: 100,
        }
    }

    /// Update path metrics
    pub fn update_path_metrics(
        &self,
        path: NetworkPath,
        rtt_ms: f32,
        loss_rate: f32,
        throughput_mbps: f32,
        congestion_window: u32,
        queue_depth: usize,
        active_streams: usize,
        is_active: bool,
    ) {
        let mut metrics = self.metrics.write();
        metrics.paths.insert(
            path.as_str().to_string(),
            PathMetricsData {
                rtt_ms,
                loss_rate,
                throughput_mbps,
                congestion_window,
                queue_depth,
                active_streams,
                is_active,
            },
        );
        metrics.timestamp = chrono::Utc::now().to_rfc3339();
    }

    /// Update path utilization
    pub fn update_path_utilization(&self, path: NetworkPath, utilization: f32) {
        let mut metrics = self.metrics.write();
        metrics.path_utilization.insert(path.as_str().to_string(), utilization);
    }

    /// Record handover event
    pub fn record_handover(&self, event: HandoverEvent) {
        let mut history = self.handover_history.write();
        history.push(event.clone());
        
        // Limit history size
        if history.len() > self.max_handover_history {
            history.remove(0);
        }

        let mut metrics = self.metrics.write();
        metrics.recent_handovers.push(HandoverEventData {
            timestamp: chrono::Utc::now().to_rfc3339(),
            from_path: event.from_path.as_str().to_string(),
            to_path: event.to_path.as_str().to_string(),
            reason: format!("{:?}", event.reason),
            priority_streams_moved: event.priority_streams_moved,
            bulk_streams_moved: event.bulk_streams_moved,
        });

        // Keep only recent handovers
        if metrics.recent_handovers.len() > 20 {
            metrics.recent_handovers.remove(0);
        }
    }

    /// Update FEC statistics
    pub fn update_fec_stats(&self, stats: FecStats) {
        let mut metrics = self.metrics.write();
        metrics.fec_stats = FecStatsData {
            total_blocks: stats.total_blocks,
            repaired_blocks: stats.repaired_blocks,
            failed_blocks: stats.failed_blocks,
            repair_rate: stats.repair_rate,
            parity_generation_rate: stats.parity_generation_rate,
        };
    }

    /// Update checksum failures
    pub fn update_checksum_failures(&self, count: u64) {
        let mut metrics = self.metrics.write();
        metrics.checksum_failures = count;
    }

    /// Update queue depths
    pub fn update_queue_depths(&self, depths: HashMap<PacketPriority, usize>) {
        let mut metrics = self.metrics.write();
        metrics.queue_depths.clear();
        for (priority, depth) in depths {
            metrics.queue_depths.insert(priority.as_str().to_string(), depth);
        }
    }

    /// Update scheduler statistics
    pub fn update_scheduler_stats(&self, stats: SchedulerStats) {
        let mut metrics = self.metrics.write();
        metrics.scheduler_stats = SchedulerStatsData {
            packets_scheduled: stats.packets_scheduled,
            packets_per_priority: stats
                .packets_per_priority
                .iter()
                .map(|(p, c)| (p.as_str().to_string(), *c))
                .collect(),
            packets_per_path: stats
                .packets_per_path
                .iter()
                .map(|(p, c)| (p.as_str().to_string(), *c))
                .collect(),
            handover_events: stats.handover_events,
            path_failures: stats.path_failures,
            total_bytes_scheduled: stats.total_bytes_scheduled,
        };
    }

    /// Update receiver statistics
    pub fn update_receiver_stats(&self, stats: ReceiverStats) {
        let mut metrics = self.metrics.write();
        metrics.receiver_stats = ReceiverStatsData {
            packets_received: stats.packets_received,
            packets_reassembled: stats.packets_reassembled,
            checksum_failures: stats.checksum_failures,
            fec_recovered: stats.fec_recovered,
            decompression_failures: stats.decompression_failures,
            total_bytes_received: stats.total_bytes_received,
            total_bytes_decompressed: stats.total_bytes_decompressed,
        };
    }

    /// Get current metrics as JSON
    pub fn to_json(&self) -> anyhow::Result<String> {
        let metrics = self.metrics.read();
        serde_json::to_string_pretty(&*metrics)
            .map_err(|e| anyhow::anyhow!("JSON serialization failed: {}", e))
    }

    /// Get current metrics
    pub fn get_metrics(&self) -> MultipathMetrics {
        self.metrics.read().clone()
    }
}

impl Default for MetricsEmitter {
    fn default() -> Self {
        Self::new()
    }
}

