//! Example: Integrating QUIC-FEC with Telemetry AI System
//! Shows how to use QUIC-FEC for telemetry transfer with automatic handover

use anyhow::Result;
use bytes::Bytes;
use quic_fec::connection::{ConnectionConfig, QuicFecConnection};
use quic_fec::handover::{NetworkPath, HandoverStrategy, PathMetrics};
use quic_fec::fec::FecConfig;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("QUIC-FEC Telemetry Integration Example");

    // Configure QUIC-FEC for telemetry transfer
    let config = ConnectionConfig {
        fec_config: FecConfig::for_telemetry(),
        handover_strategy: HandoverStrategy::Smooth,
        initial_path: NetworkPath::WiFi,
        enable_fec: true,
        max_retransmissions: 3,
    };

    // Connect to server (replace with actual server address)
    let server_addr: SocketAddr = "127.0.0.1:4433".parse()?;
    let connection = QuicFecConnection::connect(
        server_addr,
        "localhost",
        config,
    ).await?;

    println!("Connected to server");

    // Simulate telemetry data
    let telemetry_data = Bytes::from("Sample telemetry data: temperature=25.5, humidity=60%");

    // Update network metrics (simulating WiFi and 5G available)
    connection.update_path_metrics(PathMetrics {
        path: NetworkPath::WiFi,
        rtt_ms: 30.0,
        jitter_ms: 5.0,
        loss_rate: 0.02,
        throughput_mbps: 50.0,
        signal_strength: -70.0,
        last_updated: std::time::Instant::now(),
    });

    connection.update_path_metrics(PathMetrics {
        path: NetworkPath::FiveG,
        rtt_ms: 20.0,
        jitter_ms: 3.0,
        loss_rate: 0.01,
        throughput_mbps: 150.0,
        signal_strength: -65.0,
        last_updated: std::time::Instant::now(),
    });

    // Send telemetry data with FEC
    println!("Sending telemetry data with FEC...");
    connection.send(telemetry_data).await?;
    println!("Data sent successfully");

    // Check for handover (5G is better, should handover)
    if connection.check_handover().await? {
        println!("Handover performed to better network path");
    }

    // Receive response (if any)
    if let Some(data) = connection.recv().await? {
        println!("Received response: {:?}", data);
    }

    Ok(())
}

