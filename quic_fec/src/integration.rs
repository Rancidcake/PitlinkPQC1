//! Integration module for QUIC-FEC with telemetry AI system
//! Provides seamless integration between QUIC-FEC and the brain telemetry AI

use anyhow::Result;
use bytes::Bytes;
use crate::connection::{QuicFecConnection, ConnectionConfig};
use crate::handover::{NetworkPath, PathMetrics};

/// QUIC-FEC integration with telemetry AI
/// This adapter connects the telemetry AI decision system with QUIC-FEC transport
pub struct TelemetryQuicAdapter {
    connection: QuicFecConnection,
}

impl TelemetryQuicAdapter {
    /// Create a new adapter with QUIC-FEC connection
    pub async fn new(
        server_addr: std::net::SocketAddr,
        server_name: &str,
        config: ConnectionConfig,
    ) -> Result<Self> {
        let connection = QuicFecConnection::connect(server_addr, server_name, config).await?;

        Ok(Self {
            connection,
        })
    }

    /// Send telemetry data using QUIC-FEC
    /// Automatically adapts FEC based on network conditions
    pub async fn send_telemetry(&self, data: Bytes) -> Result<()> {
        self.connection.send(data).await
    }

    /// Update network metrics from telemetry AI decisions
    pub fn update_network_metrics(
        &self,
        wifi_signal: f32,
        fiveg_signal: Option<f32>,
        starlink_latency: Option<f32>,
        rtt_ms: f32,
        jitter_ms: f32,
        loss_rate: f32,
        throughput_mbps: f32,
    ) {
        // Update WiFi metrics
        self.connection.update_path_metrics(PathMetrics {
            path: NetworkPath::WiFi,
            rtt_ms,
            jitter_ms,
            loss_rate,
            throughput_mbps,
            signal_strength: wifi_signal,
            last_updated: std::time::Instant::now(),
        });

        // Update 5G metrics if available
        if let Some(signal) = fiveg_signal {
            self.connection.update_path_metrics(PathMetrics {
                path: NetworkPath::FiveG,
                rtt_ms,
                jitter_ms,
                loss_rate,
                throughput_mbps,
                signal_strength: signal,
                last_updated: std::time::Instant::now(),
            });
        }

        // Update Starlink metrics if available
        if let Some(latency) = starlink_latency {
            self.connection.update_path_metrics(PathMetrics {
                path: NetworkPath::Starlink,
                rtt_ms: latency,
                jitter_ms,
                loss_rate,
                throughput_mbps,
                signal_strength: latency,  // Use latency as signal metric
                last_updated: std::time::Instant::now(),
            });
        }
    }

    /// Check and perform handover if needed
    pub async fn check_and_handover(&self) -> Result<bool> {
        self.connection.check_handover().await
    }

    /// Receive telemetry data
    pub async fn recv_telemetry(&self) -> Result<Option<Bytes>> {
        self.connection.recv().await
    }

    /// Get current network path
    pub fn current_path(&self) -> NetworkPath {
        self.connection.handover_manager().current_path()
    }
}

