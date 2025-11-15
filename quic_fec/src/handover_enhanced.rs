//! Enhanced path handover logic with RTT spike and loss detection
//!
//! Monitors paths in real-time and triggers handover when:
//! - RTT increases by >40%
//! - Loss >7% over last 200ms
//! - Path goes down
//! - Maintains in-flight packet tracking

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Instant, Duration};
use anyhow::Result;

use crate::handover::{NetworkPath, PathMetrics, HandoverStrategy};
use crate::scheduler::PacketPriority;

/// Path monitoring window
const MONITORING_WINDOW_MS: u64 = 200;
const RTT_SPIKE_THRESHOLD: f32 = 0.4; // 40% increase
const LOSS_THRESHOLD: f32 = 0.07; // 7% loss

/// Historical path metrics for trend analysis
#[derive(Debug, Clone)]
struct PathHistory {
    rtt_samples: VecDeque<(Instant, f32)>,
    loss_samples: VecDeque<(Instant, f32)>,
    throughput_samples: VecDeque<(Instant, f32)>,
    baseline_rtt: f32,
    last_handover: Option<Instant>,
}

impl PathHistory {
    fn new() -> Self {
        Self {
            rtt_samples: VecDeque::new(),
            loss_samples: VecDeque::new(),
            throughput_samples: VecDeque::new(),
            baseline_rtt: 0.0,
            last_handover: None,
        }
    }

    /// Add RTT sample
    fn add_rtt(&mut self, rtt: f32) {
        let now = Instant::now();
        self.rtt_samples.push_back((now, rtt));
        
        // Keep only last 200ms
        let cutoff = now - Duration::from_millis(MONITORING_WINDOW_MS);
        self.rtt_samples.retain(|(t, _)| *t > cutoff);

        // Update baseline (median of recent samples)
        if self.rtt_samples.len() > 10 {
            let mut sorted: Vec<f32> = self.rtt_samples.iter().map(|(_, r)| *r).collect();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            self.baseline_rtt = sorted[sorted.len() / 2];
        } else if self.baseline_rtt == 0.0 {
            self.baseline_rtt = rtt;
        }
    }

    /// Add loss sample
    fn add_loss(&mut self, loss: f32) {
        let now = Instant::now();
        self.loss_samples.push_back((now, loss));
        
        let cutoff = now - Duration::from_millis(MONITORING_WINDOW_MS);
        self.loss_samples.retain(|(t, _)| *t > cutoff);
    }

    /// Get average loss over monitoring window
    fn avg_loss(&self) -> f32 {
        if self.loss_samples.is_empty() {
            return 0.0;
        }
        self.loss_samples.iter().map(|(_, l)| *l).sum::<f32>() / self.loss_samples.len() as f32
    }

    /// Check if RTT has spiked
    fn has_rtt_spike(&self, current_rtt: f32) -> bool {
        if self.baseline_rtt == 0.0 {
            return false;
        }
        let increase = (current_rtt - self.baseline_rtt) / self.baseline_rtt;
        increase > RTT_SPIKE_THRESHOLD
    }
}

/// In-flight packet tracking
#[derive(Debug, Clone)]
struct InFlightPacket {
    stream_id: u64,
    sequence: u64,
    priority: PacketPriority,
    sent_at: Instant,
    path: NetworkPath,
}

/// Enhanced handover manager
pub struct EnhancedHandoverManager {
    paths: Arc<RwLock<HashMap<NetworkPath, PathHistory>>>,
    current_path: Arc<RwLock<NetworkPath>>,
    strategy: HandoverStrategy,
    in_flight: Arc<RwLock<HashMap<NetworkPath, Vec<InFlightPacket>>>>,
    handover_events: Arc<RwLock<Vec<HandoverEvent>>>,
}

/// Handover event
#[derive(Debug, Clone)]
pub struct HandoverEvent {
    pub timestamp: Instant,
    pub from_path: NetworkPath,
    pub to_path: NetworkPath,
    pub reason: HandoverReason,
    pub priority_streams_moved: usize,
    pub bulk_streams_moved: usize,
}

/// Handover reason
#[derive(Debug, Clone)]
pub enum HandoverReason {
    RttSpike,
    HighLoss,
    PathDown,
    BetterPathAvailable,
    Manual,
}

impl EnhancedHandoverManager {
    /// Create new enhanced handover manager
    pub fn new(initial_path: NetworkPath, strategy: HandoverStrategy) -> Self {
        let mut paths = HashMap::new();
        paths.insert(initial_path, PathHistory::new());

        Self {
            paths: Arc::new(RwLock::new(paths)),
            current_path: Arc::new(RwLock::new(initial_path)),
            strategy,
            in_flight: Arc::new(RwLock::new(HashMap::new())),
            handover_events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Update path metrics
    pub fn update_path_metrics(&self, path: NetworkPath, metrics: PathMetrics) {
        let mut paths = self.paths.write();
        let history = paths.entry(path).or_insert_with(PathHistory::new);
        
        history.add_rtt(metrics.rtt_ms);
        history.add_loss(metrics.loss_rate);
    }

    /// Check if handover is needed
    pub fn should_handover(&self, path: NetworkPath) -> Option<(NetworkPath, HandoverReason)> {
        let paths = self.paths.read();
        let current = *self.current_path.read();

        if path != current {
            return None; // Not monitoring this path
        }

        let history = paths.get(&current)?;

        // Check for RTT spike
        if let Some((_, latest_rtt)) = history.rtt_samples.back() {
            if history.has_rtt_spike(*latest_rtt) {
                if let Some(better_path) = self.find_better_path(current, &paths) {
                    return Some((better_path, HandoverReason::RttSpike));
                }
            }
        }

        // Check for high loss
        let avg_loss = history.avg_loss();
        if avg_loss > LOSS_THRESHOLD {
            if let Some(better_path) = self.find_better_path(current, &paths) {
                return Some((better_path, HandoverReason::HighLoss));
            }
        }

        // Check if path is down (no recent samples)
        if history.rtt_samples.is_empty() || 
           history.rtt_samples.back().unwrap().0.elapsed() > Duration::from_secs(5) {
            if let Some(better_path) = self.find_better_path(current, &paths) {
                return Some((better_path, HandoverReason::PathDown));
            }
        }

        None
    }

    /// Find better path than current
    fn find_better_path(
        &self,
        current: NetworkPath,
        paths: &HashMap<NetworkPath, PathHistory>,
    ) -> Option<NetworkPath> {
        let current_history = paths.get(&current)?;
        let current_rtt = current_history.baseline_rtt;
        let current_loss = current_history.avg_loss();

        let mut best_path = None;
        let mut best_score = -1.0;

        for (path, history) in paths.iter() {
            if *path == current {
                continue;
            }

            let rtt = history.baseline_rtt;
            let loss = history.avg_loss();

            // Score: lower RTT and loss is better
            let rtt_score = if current_rtt > 0.0 {
                (current_rtt - rtt) / current_rtt
            } else {
                0.0
            };
            let loss_score = (current_loss - loss).max(0.0);
            let score = rtt_score * 0.7 + loss_score * 0.3;

            if score > best_score && score > 0.1 {
                best_score = score;
                best_path = Some(*path);
            }
        }

        best_path
    }

    /// Perform handover
    pub fn perform_handover(
        &self,
        to_path: NetworkPath,
        reason: HandoverReason,
    ) -> Result<HandoverEvent> {
        let from_path = *self.current_path.read();
        
        // Move priority streams first
        let mut in_flight = self.in_flight.write();
        let priority_streams = self.move_priority_streams(&mut in_flight, from_path, to_path);
        let bulk_streams = self.move_bulk_streams(&mut in_flight, from_path, to_path);

        // Update current path
        *self.current_path.write() = to_path;

        // Record event
        let event = HandoverEvent {
            timestamp: Instant::now(),
            from_path,
            to_path,
            reason,
            priority_streams_moved: priority_streams,
            bulk_streams_moved: bulk_streams,
        };

        self.handover_events.write().push(event.clone());

        // Update history
        if let Some(history) = self.paths.write().get_mut(&to_path) {
            history.last_handover = Some(Instant::now());
        }

        Ok(event)
    }

    /// Move priority streams to new path
    fn move_priority_streams(
        &self,
        in_flight: &mut HashMap<NetworkPath, Vec<InFlightPacket>>,
        from: NetworkPath,
        to: NetworkPath,
    ) -> usize {
        // First, collect packets to move
        let mut packets_to_move = Vec::new();
        let mut remaining = Vec::new();
        
        if let Some(from_packets) = in_flight.get_mut(&from) {
            for packet in from_packets.drain(..) {
                match packet.priority {
                    PacketPriority::Critical | PacketPriority::High => {
                        let mut moved_packet = packet;
                        moved_packet.path = to;
                        packets_to_move.push(moved_packet);
                    }
                    _ => {
                        remaining.push(packet);
                    }
                }
            }
            *from_packets = remaining;
        }
        
        let moved_count = packets_to_move.len();
        
        // Now add to destination (separate borrow)
        if !packets_to_move.is_empty() {
            let to_packets = in_flight.entry(to).or_insert_with(Vec::new);
            to_packets.extend(packets_to_move);
        }
        
        moved_count
    }

    /// Move bulk streams to new path
    fn move_bulk_streams(
        &self,
        in_flight: &mut HashMap<NetworkPath, Vec<InFlightPacket>>,
        from: NetworkPath,
        to: NetworkPath,
    ) -> usize {
        // First, collect packets to move
        let mut packets_to_move = Vec::new();
        let mut remaining = Vec::new();
        
        if let Some(from_packets) = in_flight.get_mut(&from) {
            for packet in from_packets.drain(..) {
                match packet.priority {
                    PacketPriority::Bulk => {
                        let mut moved_packet = packet;
                        moved_packet.path = to;
                        packets_to_move.push(moved_packet);
                    }
                    _ => {
                        remaining.push(packet);
                    }
                }
            }
            *from_packets = remaining;
        }
        
        let moved_count = packets_to_move.len();
        
        // Now add to destination (separate borrow)
        if !packets_to_move.is_empty() {
            let to_packets = in_flight.entry(to).or_insert_with(Vec::new);
            to_packets.extend(packets_to_move);
        }
        
        moved_count
    }

    /// Track in-flight packet
    pub fn track_packet(
        &self,
        path: NetworkPath,
        stream_id: u64,
        sequence: u64,
        priority: PacketPriority,
    ) {
        let mut in_flight = self.in_flight.write();
        let packets = in_flight.entry(path).or_insert_with(Vec::new);
        
        packets.push(InFlightPacket {
            stream_id,
            sequence,
            priority,
            sent_at: Instant::now(),
            path,
        });
    }

    /// Mark packet as received/acknowledged
    pub fn mark_received(&self, path: NetworkPath, stream_id: u64, sequence: u64) {
        let mut in_flight = self.in_flight.write();
        if let Some(packets) = in_flight.get_mut(&path) {
            packets.retain(|p| !(p.stream_id == stream_id && p.sequence == sequence));
        }
    }

    /// Get current path
    pub fn current_path(&self) -> NetworkPath {
        *self.current_path.read()
    }

    /// Get handover events
    pub fn handover_events(&self) -> Vec<HandoverEvent> {
        self.handover_events.read().clone()
    }

    /// Get in-flight packet count per path
    pub fn in_flight_counts(&self) -> HashMap<NetworkPath, usize> {
        let in_flight = self.in_flight.read();
        in_flight
            .iter()
            .map(|(path, packets)| (*path, packets.len()))
            .collect()
    }
}

