//! QUIC packet receiver with checksum verification, reassembly, and LZ4 decompression
//!
//! Handles:
//! - QUIC packet checksum verification
//! - FEC recovery for missing/failed packets
//! - Packet reordering and reassembly
//! - LZ4 decompression
//! - Stream reconstruction

use anyhow::{Result, Context};
use bytes::{Bytes, BytesMut};
use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Instant, Duration};

use common::blake3_hash;
use lz4_flex::decompress;

use crate::packet::{QuicFecPacket, PacketHeader, PacketType};
use crate::fec_enhanced::{EnhancedFecDecoder, FecBlockInfo, FecAlgorithm};

/// Stream reassembly state
#[derive(Debug, Clone)]
struct StreamState {
    stream_id: u64,
    expected_sequence: u64,
    received_packets: BTreeMap<u64, Bytes>, // sequence -> data
    reassembled: Option<Bytes>,
    last_update: Instant,
}

/// QUIC receiver with FEC and decompression
pub struct QuicReceiver {
    /// FEC decoder
    fec_decoder: Arc<EnhancedFecDecoder>,
    
    /// Stream states
    streams: Arc<RwLock<HashMap<u64, StreamState>>>,
    
    /// Checksum failures
    checksum_failures: Arc<RwLock<u64>>,
    
    /// FEC recovery statistics
    fec_recovered: Arc<RwLock<u64>>,
    
    /// Statistics
    stats: Arc<RwLock<ReceiverStats>>,
}

/// Receiver statistics
#[derive(Debug, Clone, Default)]
pub struct ReceiverStats {
    pub packets_received: u64,
    pub packets_reassembled: u64,
    pub checksum_failures: u64,
    pub fec_recovered: u64,
    pub decompression_failures: u64,
    pub total_bytes_received: u64,
    pub total_bytes_decompressed: u64,
}

impl QuicReceiver {
    /// Create new receiver
    pub fn new(fec_algorithm: FecAlgorithm, fec_config: crate::fec::FecConfig) -> Result<Self> {
        let fec_decoder = Arc::new(
            EnhancedFecDecoder::new(fec_algorithm, fec_config)
                .context("Failed to create FEC decoder")?
        );

        Ok(Self {
            fec_decoder,
            streams: Arc::new(RwLock::new(HashMap::new())),
            checksum_failures: Arc::new(RwLock::new(0)),
            fec_recovered: Arc::new(RwLock::new(0)),
            stats: Arc::new(RwLock::new(ReceiverStats::default())),
        })
    }

    /// Process received packet
    pub fn receive_packet(&self, packet_data: Bytes) -> Result<Option<Bytes>> {
        // Parse packet
        let packet = QuicFecPacket::from_bytes(&packet_data)
            .context("Failed to parse QUIC packet")?;

        // Verify checksum
        if !self.verify_checksum(&packet) {
            *self.checksum_failures.write() += 1;
            self.stats.write().checksum_failures += 1;
            
            // Try FEC recovery
            return self.attempt_fec_recovery(&packet);
        }

        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.packets_received += 1;
            stats.total_bytes_received += packet_data.len() as u64;
        }

        // Handle based on packet type
        match packet.header.packet_type {
            PacketType::Data => {
                self.handle_data_packet(packet)
            }
            PacketType::FecParity => {
                self.handle_fec_packet(packet)?;
                Ok(None) // FEC packets don't produce output directly
            }
            PacketType::Handover | PacketType::Ack | PacketType::Heartbeat => {
                // Handover signal and other control packets - just acknowledge
                Ok(None)
            }
        }
    }

    /// Verify packet checksum
    fn verify_checksum(&self, packet: &QuicFecPacket) -> bool {
        let computed = blake3_hash(&packet.data);
        computed[..16] == packet.header.checksum
    }

    /// Attempt FEC recovery for failed packet
    fn attempt_fec_recovery(&self, packet: &QuicFecPacket) -> Result<Option<Bytes>> {
        // Add to FEC decoder if it's a data shard
        if packet.header.packet_type == PacketType::Data {
            let block_id = packet.header.fec_block_id as u64;
            let info = FecBlockInfo {
                block_id,
                algorithm: FecAlgorithm::ReedSolomon, // Default
                data_shards: packet.header.total_shards as usize,
                parity_shards: 0, // Will be determined by decoder
                shard_size: packet.data.len(),
                total_size: 0, // Unknown
            };

            self.fec_decoder.add_shard(
                block_id,
                packet.header.shard_index as usize,
                packet.data.clone(),
                info,
            )?;

            // Try to decode
            if let Some(decoded) = self.fec_decoder.decode(block_id)? {
                *self.fec_recovered.write() += 1;
                self.stats.write().fec_recovered += 1;
                
                // Process decoded block
                return self.process_recovered_block(decoded, block_id);
            }
        }

        Ok(None)
    }

    /// Process FEC-recovered block
    fn process_recovered_block(&self, block_data: Bytes, block_id: u64) -> Result<Option<Bytes>> {
        // The recovered block might contain multiple packets
        // For simplicity, treat it as a single packet
        // In production, you'd parse the block structure
        
        // Try to parse as QUIC packet
        if let Ok(packet) = QuicFecPacket::from_bytes(&block_data) {
            if self.verify_checksum(&packet) {
                return self.handle_data_packet(packet);
            }
        }

        Ok(None)
    }

    /// Handle data packet
    fn handle_data_packet(&self, packet: QuicFecPacket) -> Result<Option<Bytes>> {
        let stream_id = packet.header.sequence; // Using sequence as stream ID for simplicity
        let sequence = packet.header.sequence;

        let mut streams = self.streams.write();
        let stream = streams.entry(stream_id).or_insert_with(|| StreamState {
            stream_id,
            expected_sequence: 0,
            received_packets: BTreeMap::new(),
            reassembled: None,
            last_update: Instant::now(),
        });

        // Store packet
        stream.received_packets.insert(sequence, packet.data.clone());
        stream.last_update = Instant::now();

        // Try to reassemble if we have consecutive packets
        if let Some(reassembled) = self.try_reassemble_stream(stream) {
            stream.reassembled = Some(reassembled.clone());
            self.stats.write().packets_reassembled += 1;
            
            // Decompress
            return self.decompress_and_return(reassembled);
        }

        Ok(None)
    }

    /// Handle FEC parity packet
    fn handle_fec_packet(&self, packet: QuicFecPacket) -> Result<()> {
        let block_id = packet.header.fec_block_id as u64;
        let info = FecBlockInfo {
            block_id,
            algorithm: FecAlgorithm::ReedSolomon,
            data_shards: packet.header.total_shards as usize,
            parity_shards: 1, // At least one parity shard
            shard_size: packet.data.len(),
            total_size: 0,
        };

        self.fec_decoder.add_shard(
            block_id,
            packet.header.shard_index as usize,
            packet.data.clone(),
            info,
        )?;

        Ok(())
    }

    /// Try to reassemble stream from received packets
    fn try_reassemble_stream(&self, stream: &mut StreamState) -> Option<Bytes> {
        if stream.received_packets.is_empty() {
            return None;
        }

        // Check if we have consecutive packets starting from expected_sequence
        let mut reassembled = BytesMut::new();
        let mut next_expected = stream.expected_sequence;

        loop {
            if let Some(data) = stream.received_packets.get(&next_expected) {
                reassembled.extend_from_slice(data);
                next_expected += 1;
            } else {
                break;
            }
        }

        if reassembled.is_empty() {
            return None;
        }

        stream.expected_sequence = next_expected;
        Some(reassembled.freeze())
    }

    /// Decompress LZ4 data and return
    fn decompress_and_return(&self, compressed: Bytes) -> Result<Option<Bytes>> {
        // Try LZ4 decompression
        match decompress(&compressed, compressed.len() * 4) {
            Ok(decompressed) => {
                let mut stats = self.stats.write();
                stats.total_bytes_decompressed += decompressed.len() as u64;
                Ok(Some(Bytes::from(decompressed)))
            }
            Err(e) => {
                // If decompression fails, might not be compressed
                // Return original data
                self.stats.write().decompression_failures += 1;
                Ok(Some(compressed))
            }
        }
    }

    /// Get receiver statistics
    pub fn stats(&self) -> ReceiverStats {
        self.stats.read().clone()
    }

    /// Get FEC statistics
    pub fn fec_stats(&self) -> crate::fec_enhanced::FecStats {
        self.fec_decoder.stats()
    }

    /// Clean up old streams
    pub fn cleanup_old_streams(&self, max_age_seconds: u64) {
        let mut streams = self.streams.write();
        let cutoff = Instant::now() - Duration::from_secs(max_age_seconds);
        
        streams.retain(|_, stream| stream.last_update > cutoff);
    }
}

