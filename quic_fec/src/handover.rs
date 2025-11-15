//! Network handover management for QUIC-FEC
//! Handles seamless transitions between network paths (5G/WiFi/Starlink)

use anyhow::Result;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration, Instant};

/// Network path identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NetworkPath {
    WiFi,
    FiveG,
    Starlink,
    Multipath,
}

impl NetworkPath {
    pub fn as_str(&self) -> &'static str {
        match self {
            NetworkPath::WiFi => "WiFi",
            NetworkPath::FiveG => "5G",
            NetworkPath::Starlink => "Starlink",
            NetworkPath::Multipath => "Multipath",
        }
    }
}

/// Handover strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandoverStrategy {
    /// Immediate handover (may cause brief interruption)
    Immediate,
    /// Smooth handover (maintain both connections during transition)
    Smooth,
    /// Aggressive handover (switch immediately, use FEC to recover losses)
    Aggressive,
}

/// Network path metrics
#[derive(Debug, Clone)]
pub struct PathMetrics {
    pub path: NetworkPath,
    pub rtt_ms: f32,
    pub jitter_ms: f32,
    pub loss_rate: f32,
    pub throughput_mbps: f32,
    pub signal_strength: f32,  // dBm for WiFi/5G, latency for Starlink
    pub last_updated: Instant,
}

impl PathMetrics {
    pub fn score(&self) -> f32 {
        // Calculate quality score (0.0-1.0)
        let mut score = 1.0;

        // Penalize high latency
        if self.rtt_ms > 100.0 {
            score -= 0.3;
        } else if self.rtt_ms > 50.0 {
            score -= 0.15;
        }

        // Penalize high jitter
        if self.jitter_ms > 20.0 {
            score -= 0.2;
        } else if self.jitter_ms > 10.0 {
            score -= 0.1;
        }

        // Penalize packet loss
        score -= self.loss_rate.min(0.5) * 2.0;

        // Penalize low throughput
        if self.throughput_mbps < 10.0 {
            score -= 0.3;
        } else if self.throughput_mbps < 50.0 {
            score -= 0.15;
        }

        // Penalize weak signal (for WiFi/5G)
        if matches!(self.path, NetworkPath::WiFi | NetworkPath::FiveG) {
            if self.signal_strength < -90.0 {
                score -= 0.3;
            } else if self.signal_strength < -80.0 {
                score -= 0.15;
            }
        }

        score.max(0.0).min(1.0)
    }
}

/// Handover manager - manages network path transitions
pub struct HandoverManager {
    current_path: Arc<RwLock<NetworkPath>>,
    available_paths: Arc<RwLock<Vec<PathMetrics>>>,
    strategy: HandoverStrategy,
    handover_in_progress: Arc<RwLock<bool>>,
    last_handover: Arc<RwLock<Option<Instant>>>,
}

impl HandoverManager {
    /// Create a new handover manager
    pub fn new(initial_path: NetworkPath, strategy: HandoverStrategy) -> Self {
        Self {
            current_path: Arc::new(RwLock::new(initial_path)),
            available_paths: Arc::new(RwLock::new(Vec::new())),
            strategy,
            handover_in_progress: Arc::new(RwLock::new(false)),
            last_handover: Arc::new(RwLock::new(None)),
        }
    }

    /// Update metrics for a network path
    pub fn update_path_metrics(&self, metrics: PathMetrics) {
        let mut paths = self.available_paths.write();
        
        // Update or insert metrics
        if let Some(existing) = paths.iter_mut().find(|p| p.path == metrics.path) {
            *existing = metrics;
        } else {
            paths.push(metrics);
        }
    }

    /// Get current network path
    pub fn current_path(&self) -> NetworkPath {
        *self.current_path.read()
    }

    /// Check if handover is needed and return recommended path
    pub fn should_handover(&self) -> Option<NetworkPath> {
        let current = *self.current_path.read();
        let paths = self.available_paths.read();

        // Get current path metrics
        let current_metrics = paths.iter()
            .find(|p| p.path == current)
            .cloned();

        // Find best available path
        let best_path = paths.iter()
            .max_by(|a, b| {
                let score_a = a.score();
                let score_b = b.score();
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()?;

        // Check if handover is needed
        let current_score = current_metrics.as_ref().map(|m| m.score()).unwrap_or(0.0);
        let best_score = best_path.score();

        // Handover if:
        // 1. Best path is significantly better (20% improvement)
        // 2. Current path is very poor (< 0.3)
        // 3. Best path is different from current
        if best_path.path != current && (
            best_score > current_score * 1.2 ||
            current_score < 0.3 ||
            best_score > 0.7 && current_score < 0.5
        ) {
            // Prevent rapid handovers (min 5 seconds between handovers)
            if let Some(last) = *self.last_handover.read() {
                if last.elapsed() < Duration::from_secs(5) {
                    return None;
                }
            }

            Some(best_path.path)
        } else {
            None
        }
    }

    /// Execute handover to new path
    pub fn handover_to(&self, new_path: NetworkPath) -> Result<()> {
        let current = *self.current_path.read();
        
        if new_path == current {
            return Ok(());  // Already on this path
        }

        // Check if handover already in progress
        {
            let mut in_progress = self.handover_in_progress.write();
            if *in_progress {
                return Ok(());  // Handover already in progress
            }
            *in_progress = true;
        }

        // Update current path
        {
            let mut path = self.current_path.write();
            *path = new_path;
        }

        // Update last handover time
        {
            let mut last = self.last_handover.write();
            *last = Some(Instant::now());
        }

        // Mark handover complete
        {
            let mut in_progress = self.handover_in_progress.write();
            *in_progress = false;
        }

        Ok(())
    }

    /// Get recommended FEC configuration based on current network conditions
    pub fn recommended_fec_config(&self) -> crate::fec::FecConfig {
        let current = *self.current_path.read();
        let paths = self.available_paths.read();

        let metrics = paths.iter()
            .find(|p| p.path == current)
            .cloned();

        if let Some(metrics) = metrics {
            let score = metrics.score();
            
            if score < 0.3 {
                // Very poor network - use high redundancy
                crate::fec::FecConfig::for_patchy_network()
            } else if score < 0.6 {
                // Poor network - use telemetry config
                crate::fec::FecConfig::for_telemetry()
            } else {
                // Good network - use default
                crate::fec::FecConfig::default()
            }
        } else {
            // Unknown network - use conservative config
            crate::fec::FecConfig::for_telemetry()
        }
    }

    /// Check if handover is currently in progress
    pub fn is_handover_in_progress(&self) -> bool {
        *self.handover_in_progress.read()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_metrics_score() {
        let metrics = PathMetrics {
            path: NetworkPath::WiFi,
            rtt_ms: 20.0,
            jitter_ms: 5.0,
            loss_rate: 0.01,
            throughput_mbps: 100.0,
            signal_strength: -60.0,
            last_updated: Instant::now(),
        };

        let score = metrics.score();
        assert!(score > 0.5);  // Should be decent score
    }

    #[test]
    fn test_handover_decision() {
        let manager = HandoverManager::new(NetworkPath::WiFi, HandoverStrategy::Smooth);

        // Add poor WiFi metrics
        manager.update_path_metrics(PathMetrics {
            path: NetworkPath::WiFi,
            rtt_ms: 150.0,
            jitter_ms: 30.0,
            loss_rate: 0.1,
            throughput_mbps: 5.0,
            signal_strength: -95.0,
            last_updated: Instant::now(),
        });

        // Add good 5G metrics
        manager.update_path_metrics(PathMetrics {
            path: NetworkPath::FiveG,
            rtt_ms: 20.0,
            jitter_ms: 5.0,
            loss_rate: 0.01,
            throughput_mbps: 150.0,
            signal_strength: -65.0,
            last_updated: Instant::now(),
        });

        // Should recommend handover to 5G
        let recommended = manager.should_handover();
        assert_eq!(recommended, Some(NetworkPath::FiveG));
    }
}

