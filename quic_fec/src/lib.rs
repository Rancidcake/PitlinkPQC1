//! QUIC-FEC: Modified QUIC protocol with Forward Error Correction (FEC)
//! Optimized for telemetry transfer and network handover scenarios
//!
//! This module provides:
//! - QUIC connection management with automatic handover support
//! - Forward Error Correction (FEC) using Reed-Solomon erasure coding
//! - Blake3 hashing for integrity verification
//! - Optimized for telemetry data transfer with low latency
//! - Seamless handover between network paths (5G/WiFi/Starlink)

mod fec;
mod connection;
mod handover;
mod packet;
mod integration;
mod scheduler;
mod fec_enhanced;
mod handover_enhanced;
mod receiver;
mod metrics;

pub use fec::{FecEncoder, FecDecoder, FecConfig};
pub use connection::{QuicFecConnection, ConnectionConfig, ConnectionState};
pub use handover::{HandoverManager, HandoverStrategy, NetworkPath, PathMetrics};
pub use packet::{QuicFecPacket, PacketType, PacketHeader};
pub use integration::TelemetryQuicAdapter;

// Enhanced multipath exports
pub use scheduler::{MultipathScheduler, PacketPriority, ScheduledPacket, PathStats, SchedulerStats};
pub use fec_enhanced::{EnhancedFecEncoder, EnhancedFecDecoder, FecAlgorithm, FecStats, FecBlockInfo};
pub use handover_enhanced::{EnhancedHandoverManager, HandoverEvent, HandoverReason};
pub use receiver::{QuicReceiver, ReceiverStats};
pub use metrics::{MultipathMetrics, MetricsEmitter};

use anyhow::Result;

/// Initialize QUIC-FEC system
pub fn init() -> Result<()> {
    // Initialize any global state if needed
    Ok(())
}

