//! Priority-aware multipath scheduler for P-QUIC
//!
//! This module implements intelligent packet scheduling across multiple network paths
//! with priority-based routing and dynamic path selection.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Instant, Duration};
use bytes::Bytes;
use anyhow::{Result, Context};

use crate::handover::{NetworkPath, PathMetrics};

/// Packet priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PacketPriority {
    /// Critical: Lowest latency, highest reliability
    Critical = 0,
    /// High: Low latency, high reliability
    High = 1,
    /// Medium: Balanced latency and throughput
    Medium = 2,
    /// Bulk: Maximum throughput
    Bulk = 3,
}

impl PacketPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            PacketPriority::Critical => "Critical",
            PacketPriority::High => "High",
            PacketPriority::Medium => "Medium",
            PacketPriority::Bulk => "Bulk",
        }
    }
}

/// Scheduled packet with metadata
#[derive(Debug, Clone)]
pub struct ScheduledPacket {
    pub priority: PacketPriority,
    pub data: Bytes,
    pub stream_id: u64,
    pub sequence: u64,
    pub path_preference: Option<NetworkPath>,
    pub timestamp: Instant,
}

/// Path statistics for scheduling decisions
#[derive(Debug, Clone)]
pub struct PathStats {
    pub path: NetworkPath,
    pub rtt_ms: f32,
    pub loss_rate: f32,
    pub throughput_mbps: f32,
    pub congestion_window: u32,
    pub queue_depth: usize,
    pub active_streams: usize,
    pub last_updated: Instant,
}

impl PathStats {
    /// Calculate weighted score for priority-based routing
    pub fn score(&self, priority: PacketPriority) -> f32 {
        match priority {
            PacketPriority::Critical => {
                // Critical: minimize RTT, ignore throughput
                let rtt_score = 1.0 / (1.0 + self.rtt_ms / 10.0);
                let loss_penalty = self.loss_rate * 10.0;
                (rtt_score - loss_penalty).max(0.0)
            }
            PacketPriority::High => {
                // High: weighted RTT + loss score
                let rtt_score = 1.0 / (1.0 + self.rtt_ms / 20.0);
                let loss_penalty = self.loss_rate * 5.0;
                let throughput_bonus = (self.throughput_mbps / 100.0).min(0.2);
                (rtt_score - loss_penalty + throughput_bonus).max(0.0)
            }
            PacketPriority::Medium => {
                // Medium: balanced score
                let rtt_score = 1.0 / (1.0 + self.rtt_ms / 50.0);
                let throughput_score = (self.throughput_mbps / 200.0).min(0.5);
                let loss_penalty = self.loss_rate * 2.0;
                (rtt_score + throughput_score - loss_penalty).max(0.0)
            }
            PacketPriority::Bulk => {
                // Bulk: maximize bandwidth
                let throughput_score = (self.throughput_mbps / 500.0).min(1.0);
                let rtt_penalty = (self.rtt_ms / 200.0).min(0.3);
                (throughput_score - rtt_penalty).max(0.0)
            }
        }
    }

    /// Check if path is healthy for the given priority
    pub fn is_healthy(&self, priority: PacketPriority) -> bool {
        match priority {
            PacketPriority::Critical => {
                self.rtt_ms < 100.0 && self.loss_rate < 0.05 && self.queue_depth < 100
            }
            PacketPriority::High => {
                self.rtt_ms < 200.0 && self.loss_rate < 0.07 && self.queue_depth < 200
            }
            PacketPriority::Medium => {
                self.rtt_ms < 500.0 && self.loss_rate < 0.10 && self.queue_depth < 500
            }
            PacketPriority::Bulk => {
                self.loss_rate < 0.15 // Bulk can tolerate higher loss
            }
        }
    }
}

/// Priority-aware multipath scheduler
pub struct MultipathScheduler {
    /// Path statistics table
    paths: Arc<RwLock<HashMap<NetworkPath, PathStats>>>,
    
    /// Priority queues for each priority level
    queues: Arc<RwLock<HashMap<PacketPriority, VecDeque<ScheduledPacket>>>>,
    
    /// Round-robin counter for medium priority
    medium_rr_counter: Arc<RwLock<usize>>,
    
    /// In-flight packet tracking (path -> stream_id -> sequence)
    in_flight: Arc<RwLock<HashMap<NetworkPath, HashMap<u64, Vec<u64>>>>>,
    
    /// Statistics
    stats: Arc<RwLock<SchedulerStats>>,
}

/// Scheduler statistics
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    pub packets_scheduled: u64,
    pub packets_per_priority: HashMap<PacketPriority, u64>,
    pub packets_per_path: HashMap<NetworkPath, u64>,
    pub handover_events: u32,
    pub path_failures: u32,
    pub total_bytes_scheduled: u64,
}

impl MultipathScheduler {
    /// Create a new multipath scheduler
    pub fn new() -> Self {
        let mut queues = HashMap::new();
        queues.insert(PacketPriority::Critical, VecDeque::new());
        queues.insert(PacketPriority::High, VecDeque::new());
        queues.insert(PacketPriority::Medium, VecDeque::new());
        queues.insert(PacketPriority::Bulk, VecDeque::new());

        Self {
            paths: Arc::new(RwLock::new(HashMap::new())),
            queues: Arc::new(RwLock::new(queues)),
            medium_rr_counter: Arc::new(RwLock::new(0)),
            in_flight: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(SchedulerStats::default())),
        }
    }

    /// Update path statistics
    pub fn update_path_stats(&self, path: NetworkPath, stats: PathStats) {
        let mut paths = self.paths.write();
        paths.insert(path, stats);
    }

    /// Schedule a packet with priority
    pub fn schedule(
        &self,
        priority: PacketPriority,
        data: Bytes,
        stream_id: u64,
        sequence: u64,
        path_preference: Option<NetworkPath>,
    ) -> Result<()> {
        let packet = ScheduledPacket {
            priority,
            data: data.clone(),
            stream_id,
            sequence,
            path_preference,
            timestamp: Instant::now(),
        };

        let mut queues = self.queues.write();
        queues.get_mut(&priority)
            .ok_or_else(|| anyhow::anyhow!("Invalid priority"))?
            .push_back(packet);

        // Update statistics
        let mut stats = self.stats.write();
        stats.packets_scheduled += 1;
        *stats.packets_per_priority.entry(priority).or_insert(0) += 1;
        stats.total_bytes_scheduled += data.len() as u64;

        Ok(())
    }

    /// Get next packet to send with selected path
    pub fn next_packet(&self) -> Option<(ScheduledPacket, NetworkPath)> {
        let paths = self.paths.read();
        let mut queues = self.queues.write();

        // Critical priority: lowest RTT
        if let Some(queue) = queues.get_mut(&PacketPriority::Critical) {
            if let Some(packet) = queue.front() {
                if let Some(path) = self.select_path_for_priority(
                    PacketPriority::Critical,
                    packet.path_preference,
                    &paths,
                ) {
                    let packet = queue.pop_front()?;
                    return Some((packet, path));
                }
            }
        }

        // High priority: weighted RTT + loss score
        if let Some(queue) = queues.get_mut(&PacketPriority::High) {
            if let Some(packet) = queue.front() {
                if let Some(path) = self.select_path_for_priority(
                    PacketPriority::High,
                    packet.path_preference,
                    &paths,
                ) {
                    let packet = queue.pop_front()?;
                    return Some((packet, path));
                }
            }
        }

        // Medium priority: round-robin
        if let Some(queue) = queues.get_mut(&PacketPriority::Medium) {
            if queue.front().is_some() {
                if let Some(path) = self.select_round_robin(&paths) {
                    let packet = queue.pop_front()?;
                    return Some((packet, path));
                }
            }
        }

        // Bulk priority: highest bandwidth
        if let Some(queue) = queues.get_mut(&PacketPriority::Bulk) {
            if let Some(packet) = queue.front() {
                if let Some(path) = self.select_path_for_priority(
                    PacketPriority::Bulk,
                    packet.path_preference,
                    &paths,
                ) {
                    let packet = queue.pop_front()?;
                    return Some((packet, path));
                }
            }
        }

        None
    }

    /// Select best path for a given priority
    fn select_path_for_priority(
        &self,
        priority: PacketPriority,
        preference: Option<NetworkPath>,
        paths: &HashMap<NetworkPath, PathStats>,
    ) -> Option<NetworkPath> {
        // Check preference first if healthy
        if let Some(pref_path) = preference {
            if let Some(stats) = paths.get(&pref_path) {
                if stats.is_healthy(priority) {
                    return Some(pref_path);
                }
            }
        }

        // Find best path by score
        let mut best_path = None;
        let mut best_score = -1.0;

        for (path, stats) in paths.iter() {
            if !stats.is_healthy(priority) {
                continue;
            }

            let score = stats.score(priority);
            if score > best_score {
                best_score = score;
                best_path = Some(*path);
            }
        }

        best_path
    }

    /// Round-robin path selection for medium priority
    fn select_round_robin(&self, paths: &HashMap<NetworkPath, PathStats>) -> Option<NetworkPath> {
        let available_paths: Vec<NetworkPath> = paths
            .iter()
            .filter(|(_, stats)| stats.is_healthy(PacketPriority::Medium))
            .map(|(path, _)| *path)
            .collect();

        if available_paths.is_empty() {
            return None;
        }

        let mut counter = self.medium_rr_counter.write();
        let selected = available_paths[*counter % available_paths.len()];
        *counter += 1;

        Some(selected)
    }

    /// Get packets for multipath aggregation (send simultaneously)
    pub fn get_multipath_batch(&self, max_packets: usize) -> Vec<(ScheduledPacket, NetworkPath)> {
        let mut batch = Vec::new();
        let paths = self.paths.read();
        let mut queues = self.queues.write();

        // Distribute packets across available paths
        let available_paths: Vec<NetworkPath> = paths
            .keys()
            .copied()
            .collect();

        if available_paths.is_empty() {
            return batch;
        }

        let mut path_index = 0;

        // Process by priority
        for priority in [
            PacketPriority::Critical,
            PacketPriority::High,
            PacketPriority::Medium,
            PacketPriority::Bulk,
        ] {
            let Some(queue) = queues.get_mut(&priority) else {
                continue;
            };
            
            while batch.len() < max_packets && !queue.is_empty() {
                if let Some(packet) = queue.front() {
                    let path = match self.select_path_for_priority(
                        priority,
                        packet.path_preference,
                        &paths,
                    ) {
                        Some(p) => p,
                        None => continue,
                    };

                    if let Some(pkt) = queue.pop_front() {
                        batch.push((pkt, path));
                        path_index = (path_index + 1) % available_paths.len();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            if batch.len() >= max_packets {
                break;
            }
        }

        batch
    }

    /// Track in-flight packet
    pub fn track_in_flight(&self, path: NetworkPath, stream_id: u64, sequence: u64) {
        let mut in_flight = self.in_flight.write();
        in_flight
            .entry(path)
            .or_insert_with(HashMap::new)
            .entry(stream_id)
            .or_insert_with(Vec::new)
            .push(sequence);
    }

    /// Mark packet as received/acknowledged
    pub fn mark_received(&self, path: NetworkPath, stream_id: u64, sequence: u64) {
        let mut in_flight = self.in_flight.write();
        if let Some(streams) = in_flight.get_mut(&path) {
            if let Some(sequences) = streams.get_mut(&stream_id) {
                sequences.retain(|&s| s != sequence);
                if sequences.is_empty() {
                    streams.remove(&stream_id);
                }
            }
        }
    }

    /// Get scheduler statistics
    pub fn stats(&self) -> SchedulerStats {
        self.stats.read().clone()
    }

    /// Get queue depths
    pub fn queue_depths(&self) -> HashMap<PacketPriority, usize> {
        let queues = self.queues.read();
        queues
            .iter()
            .map(|(priority, queue)| (*priority, queue.len()))
            .collect()
    }

    /// Get active paths
    pub fn active_paths(&self) -> Vec<NetworkPath> {
        let paths = self.paths.read();
        paths.keys().copied().collect()
    }
}

impl Default for MultipathScheduler {
    fn default() -> Self {
        Self::new()
    }
}

