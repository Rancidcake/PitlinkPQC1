//! QUIC-FEC Connection - Main connection management
//! Provides a modified QUIC implementation optimized for telemetry transfer with FEC

use anyhow::{Result, Context};
use quinn::{Connection, Endpoint};
use std::sync::Arc;
use parking_lot::RwLock;
use bytes::Bytes;
use std::net::SocketAddr;
use tokio::sync::mpsc;

use crate::fec::{FecEncoder, FecDecoder, FecConfig};
use crate::handover::{HandoverManager, NetworkPath, HandoverStrategy};
use crate::packet::QuicFecPacket;

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Handover,
    Error,
}

/// Connection configuration
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// FEC configuration
    pub fec_config: FecConfig,
    /// Handover strategy
    pub handover_strategy: HandoverStrategy,
    /// Initial network path
    pub initial_path: NetworkPath,
    /// Enable FEC (default: true)
    pub enable_fec: bool,
    /// Maximum retransmissions
    pub max_retransmissions: u32,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            fec_config: FecConfig::for_telemetry(),
            handover_strategy: HandoverStrategy::Smooth,
            initial_path: NetworkPath::WiFi,
            enable_fec: true,
            max_retransmissions: 3,
        }
    }
}

/// QUIC-FEC Connection wrapper
pub struct QuicFecConnection {
    connection: Option<Connection>,
    state: Arc<RwLock<ConnectionState>>,
    config: ConnectionConfig,
    fec_encoder: Option<FecEncoder>,
    fec_decoders: Arc<RwLock<std::collections::HashMap<u32, FecDecoder>>>,
    handover_manager: Arc<HandoverManager>,
    sequence_counter: Arc<RwLock<u64>>,
    fec_block_counter: Arc<RwLock<u32>>,
    send_tx: Option<mpsc::UnboundedSender<Bytes>>,
    recv_rx: Option<mpsc::UnboundedReceiver<Bytes>>,
}

impl QuicFecConnection {
    /// Create a new QUIC-FEC connection (client)
    pub async fn connect(
        server_addr: SocketAddr,
        server_name: &str,
        config: ConnectionConfig,
    ) -> Result<Self> {
        // Create QUIC client endpoint with default config
        // Note: In production, use proper certificate validation
        let endpoint = Endpoint::client("[::]:0".parse()?)?;
        
        // Connect to server
        // Note: quinn 0.11 uses server_name as &str directly
        let connection = endpoint
            .connect(server_addr, server_name)
            .map_err(|e| anyhow::anyhow!("Failed to create connection: {}", e))?
            .await
            .context("Failed to establish QUIC connection")?;

        // Create FEC encoder
        let fec_encoder = if config.enable_fec {
            Some(FecEncoder::new(config.fec_config.clone())?)
        } else {
            None
        };

        // Create handover manager
        let handover_manager = Arc::new(HandoverManager::new(
            config.initial_path,
            config.handover_strategy,
        ));

        // Create channels for send/receive
        let (send_tx, _send_rx) = mpsc::unbounded_channel();
        let (_recv_tx, recv_rx) = mpsc::unbounded_channel();

        Ok(Self {
            connection: Some(connection),
            state: Arc::new(RwLock::new(ConnectionState::Connected)),
            config,
            fec_encoder,
            fec_decoders: Arc::new(RwLock::new(std::collections::HashMap::new())),
            handover_manager,
            sequence_counter: Arc::new(RwLock::new(0)),
            fec_block_counter: Arc::new(RwLock::new(0)),
            send_tx: Some(send_tx),
            recv_rx: Some(recv_rx),
        })
    }

    /// Create a new QUIC-FEC connection (server)
    pub async fn accept(
        endpoint: Endpoint,
        config: ConnectionConfig,
    ) -> Result<Self> {
        // Accept incoming connection
        let incoming = endpoint.accept().await
            .ok_or_else(|| anyhow::anyhow!("No incoming connection"))?;
        
        let connection = incoming.await
            .context("Failed to accept QUIC connection")?;

        // Create FEC encoder
        let fec_encoder = if config.enable_fec {
            Some(FecEncoder::new(config.fec_config.clone())?)
        } else {
            None
        };

        // Create handover manager
        let handover_manager = Arc::new(HandoverManager::new(
            config.initial_path,
            config.handover_strategy,
        ));

        // Create channels
        let (send_tx, _send_rx) = mpsc::unbounded_channel();
        let (_recv_tx, recv_rx) = mpsc::unbounded_channel();

        Ok(Self {
            connection: Some(connection),
            state: Arc::new(RwLock::new(ConnectionState::Connected)),
            config,
            fec_encoder,
            fec_decoders: Arc::new(RwLock::new(std::collections::HashMap::new())),
            handover_manager,
            sequence_counter: Arc::new(RwLock::new(0)),
            fec_block_counter: Arc::new(RwLock::new(0)),
            send_tx: Some(send_tx),
            recv_rx: Some(recv_rx),
        })
    }

    /// Send data with FEC encoding
    pub async fn send(&self, data: Bytes) -> Result<()> {
        let conn = self.connection.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        if self.config.enable_fec {
            self.send_with_fec(conn, data).await
        } else {
            self.send_without_fec(conn, data).await
        }
    }

    /// Send data with FEC encoding
    async fn send_with_fec(&self, conn: &Connection, data: Bytes) -> Result<()> {
        let encoder = self.fec_encoder.as_ref()
            .ok_or_else(|| anyhow::anyhow!("FEC encoder not initialized"))?;

        // Encode data into shards
        let (data_shards, parity_shards) = encoder.encode(&data)?;

        // Get FEC block ID
        let fec_block_id = {
            let mut counter = self.fec_block_counter.write();
            *counter += 1;
            *counter
        };

        let total_shards = encoder.total_shards();

        // Send data shards
        for (index, shard) in data_shards.iter().enumerate() {
            let sequence = {
                let mut seq = self.sequence_counter.write();
                *seq += 1;
                *seq
            };

            let packet = QuicFecPacket::new_data(
                sequence,
                fec_block_id,
                index as u16,
                total_shards as u16,
                shard.clone(),
            );

            let packet_bytes = packet.to_bytes();
            let mut send_stream = conn.open_uni().await?;
            send_stream.write_all(&packet_bytes).await
                .map_err(|e| anyhow::anyhow!("Failed to write packet: {}", e))?;
            send_stream.finish()
                .map_err(|e| anyhow::anyhow!("Failed to finish stream: {}", e))?;
        }

        // Send parity shards
        for (index, shard) in parity_shards.iter().enumerate() {
            let sequence = {
                let mut seq = self.sequence_counter.write();
                *seq += 1;
                *seq
            };

            let packet = QuicFecPacket::new_fec_parity(
                sequence,
                fec_block_id,
                (data_shards.len() + index) as u16,
                total_shards as u16,
                shard.clone(),
            );

            let packet_bytes = packet.to_bytes();
            let mut send_stream = conn.open_uni().await?;
            send_stream.write_all(&packet_bytes).await
                .map_err(|e| anyhow::anyhow!("Failed to write packet: {}", e))?;
            send_stream.finish()
                .map_err(|e| anyhow::anyhow!("Failed to finish stream: {}", e))?;
        }

        Ok(())
    }

    /// Send data without FEC encoding
    async fn send_without_fec(&self, conn: &Connection, data: Bytes) -> Result<()> {
        let sequence = {
            let mut seq = self.sequence_counter.write();
            *seq += 1;
            *seq
        };

        let packet = QuicFecPacket::new_data(
            sequence,
            0,
            0,
            1,
            data,
        );

        let packet_bytes = packet.to_bytes();
        let mut send_stream = conn.open_uni().await?;
        send_stream.write_all(&packet_bytes).await?;

        Ok(())
    }

    /// Receive data (with FEC decoding if enabled)
    pub async fn recv(&self) -> Result<Option<Bytes>> {
        let conn = self.connection.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        // Receive packet
        let mut recv_stream = conn.accept_uni().await
            .map_err(|e| anyhow::anyhow!("Failed to accept stream: {}", e))?;
        
        let mut buffer = Vec::new();
        let mut chunk = vec![0u8; 8192];
        
        // Read data from stream
        // quinn 0.11 read returns Result<Option<usize>>
        loop {
            match recv_stream.read(&mut chunk).await {
                Ok(Some(0)) => break,  // EOF
                Ok(Some(n)) if n > 0 && n <= chunk.len() => {
                    buffer.extend_from_slice(&chunk[..n]);
                }
                Ok(Some(_)) => break,  // Invalid size
                Ok(None) => break,  // No more data available
                Err(_e) => {
                    // Read error - break and return what we have
                    break;
                }
            }
        }

        if buffer.is_empty() {
            return Ok(None);
        }

        let packet = QuicFecPacket::from_bytes(&buffer)?;

        if self.config.enable_fec {
            self.recv_with_fec(packet).await
        } else {
            Ok(Some(packet.data))
        }
    }

    /// Receive and decode with FEC
    async fn recv_with_fec(&self, packet: QuicFecPacket) -> Result<Option<Bytes>> {
        let fec_block_id = packet.header.fec_block_id;
        let shard_index = packet.header.shard_index as usize;

        // Get or create decoder for this FEC block
        let decoder_key = fec_block_id;
        let ready = {
            let mut decoders = self.fec_decoders.write();
            let decoder = decoders.entry(decoder_key)
                .or_insert_with(|| {
                    let config = self.config.fec_config.clone();
                    FecDecoder::new(config).unwrap()
                });
            
            // Add shard to decoder
            decoder.add_shard(shard_index, packet.data)?
        };

        if ready {
            // Try to decode
            let decoded = {
                let mut decoders = self.fec_decoders.write();
                let decoder = decoders.get_mut(&decoder_key)
                    .ok_or_else(|| anyhow::anyhow!("Decoder not found"))?;
                decoder.decode()?
            };
            
            if let Some(decoded_data) = decoded {
                // Remove decoder for this block
                let mut decoders = self.fec_decoders.write();
                decoders.remove(&decoder_key);
                Ok(Some(decoded_data))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)  // Not enough shards yet
        }
    }

    /// Update network path metrics (for handover decisions)
    pub fn update_path_metrics(&self, metrics: crate::handover::PathMetrics) {
        self.handover_manager.update_path_metrics(metrics);
    }

    /// Check and perform handover if needed
    pub async fn check_handover(&self) -> Result<bool> {
        if let Some(new_path) = self.handover_manager.should_handover() {
            // Perform handover
            self.handover_manager.handover_to(new_path)?;
            
            // Send handover packet
            if let Some(conn) = &self.connection {
                let sequence = {
                    let mut seq = self.sequence_counter.write();
                    *seq += 1;
                    *seq
                };

                let path_info = format!("{}", new_path.as_str());
                let packet = QuicFecPacket::new_handover(sequence, path_info.as_bytes());
                let packet_bytes = packet.to_bytes();
                
                let mut send_stream = conn.open_uni().await?;
                send_stream.write_all(&packet_bytes).await
                    .map_err(|e| anyhow::anyhow!("Failed to write handover packet: {}", e))?;
                send_stream.finish()
                    .map_err(|e| anyhow::anyhow!("Failed to finish handover stream: {}", e))?;
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get current connection state
    pub fn state(&self) -> ConnectionState {
        *self.state.read()
    }

    /// Get handover manager reference
    pub fn handover_manager(&self) -> &HandoverManager {
        &self.handover_manager
    }

    /// Close connection
    pub fn close(&mut self) {
        if let Some(conn) = &self.connection {
            conn.close(0u32.into(), b"Connection closed");
        }
        *self.state.write() = ConnectionState::Disconnected;
        self.connection = None;
    }

    // Note: Client configuration is handled by quinn's default endpoint
    // In production, configure certificates via Endpoint::client_with_config()

    // Note: Server configuration would be created using quinn's server builder
    // This is a placeholder - actual implementation would use proper certificates
    // Example server setup:
    // let server_config = quinn::ServerConfig::with_crypto(...);
    // let endpoint = Endpoint::server(server_config, server_addr)?;
}

impl Drop for QuicFecConnection {
    fn drop(&mut self) {
        self.close();
    }
}

