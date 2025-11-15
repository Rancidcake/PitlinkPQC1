//! Enhanced FEC Engine with XOR and Reed-Solomon support
//!
//! Provides configurable Forward Error Correction with:
//! - XOR-based FEC (fast, simple)
//! - Reed-Solomon erasure coding (robust, configurable)

use anyhow::{Result, Context};
use bytes::{Bytes, BytesMut};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use reed_solomon_erasure::{ReedSolomon, galois_8::Field};

use crate::fec::{FecConfig, FecEncoder, FecDecoder};

/// FEC algorithm type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FecAlgorithm {
    /// XOR-based FEC (fast, simple, k+1 redundancy)
    Xor,
    /// Reed-Solomon erasure coding (robust, configurable k+r)
    ReedSolomon,
}

/// Enhanced FEC encoder with algorithm selection
pub struct EnhancedFecEncoder {
    algorithm: FecAlgorithm,
    config: FecConfig,
    rs_encoder: Option<ReedSolomon<Field>>,
    block_id: u64,
}

impl EnhancedFecEncoder {
    /// Create new FEC encoder
    pub fn new(algorithm: FecAlgorithm, config: FecConfig) -> Result<Self> {
        let rs_encoder = match algorithm {
            FecAlgorithm::ReedSolomon => {
                Some(ReedSolomon::<Field>::new(config.data_shards, config.parity_shards)?)
            }
            FecAlgorithm::Xor => None,
        };

        Ok(Self {
            algorithm,
            config,
            rs_encoder,
            block_id: 0,
        })
    }

    /// Encode a block of data
    pub fn encode(&mut self, data: &[u8]) -> Result<(Vec<Bytes>, Vec<Bytes>, FecBlockInfo)> {
        self.block_id += 1;

        match self.algorithm {
            FecAlgorithm::Xor => self.encode_xor(data),
            FecAlgorithm::ReedSolomon => self.encode_reed_solomon(data),
        }
    }

    /// XOR-based encoding
    fn encode_xor(&self, data: &[u8]) -> Result<(Vec<Bytes>, Vec<Bytes>, FecBlockInfo)> {
        // Split data into shards
        let shard_size = (data.len() + self.config.data_shards - 1) / self.config.data_shards;
        let mut data_shards = Vec::new();

        for i in 0..self.config.data_shards {
            let start = i * shard_size;
            let end = (start + shard_size).min(data.len());
            
            if start < data.len() {
                let mut shard = BytesMut::with_capacity(shard_size);
                shard.extend_from_slice(&data[start..end]);
                // Pad if needed
                if shard.len() < shard_size {
                    shard.extend_from_slice(&vec![0u8; shard_size - shard.len()]);
                }
                data_shards.push(shard.freeze());
            } else {
                // Empty shard
                data_shards.push(Bytes::from(vec![0u8; shard_size]));
            }
        }

        // Generate XOR parity shards
        let mut parity_shards = Vec::new();
        for _ in 0..self.config.parity_shards {
            let mut parity = BytesMut::with_capacity(shard_size);
            
            // XOR all data shards
            for shard in &data_shards {
                for (i, &byte) in shard.iter().enumerate() {
                    if i < parity.len() {
                        parity[i] ^= byte;
                    } else {
                        parity.extend_from_slice(&[byte]);
                    }
                }
            }
            
            parity_shards.push(parity.freeze());
        }

        let info = FecBlockInfo {
            block_id: self.block_id,
            algorithm: FecAlgorithm::Xor,
            data_shards: self.config.data_shards,
            parity_shards: self.config.parity_shards,
            shard_size,
            total_size: data.len(),
        };

        Ok((data_shards, parity_shards, info))
    }

    /// Reed-Solomon encoding
    fn encode_reed_solomon(
        &self,
        data: &[u8],
    ) -> Result<(Vec<Bytes>, Vec<Bytes>, FecBlockInfo)> {
        let encoder = self.rs_encoder.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Reed-Solomon encoder not initialized"))?;

        // Split data into shards
        let shard_size = (data.len() + self.config.data_shards - 1) / self.config.data_shards;
        let mut data_shards = Vec::new();

        for i in 0..self.config.data_shards {
            let start = i * shard_size;
            let end = (start + shard_size).min(data.len());
            
            let mut shard = BytesMut::with_capacity(shard_size);
            if start < data.len() {
                shard.extend_from_slice(&data[start..end]);
            }
            // Pad if needed
            if shard.len() < shard_size {
                shard.extend_from_slice(&vec![0u8; shard_size - shard.len()]);
            }
            data_shards.push(shard.freeze());
        }

        // Convert to format expected by reed-solomon-erasure
        let mut shards: Vec<Vec<u8>> = data_shards
            .iter()
            .map(|s| s.to_vec())
            .collect();

        // Add empty parity shards
        for _ in 0..self.config.parity_shards {
            shards.push(vec![0u8; shard_size]);
        }

        // Encode
        encoder.encode(&mut shards)
            .context("Reed-Solomon encoding failed")?;

        // Split back into data and parity
        let encoded_data: Vec<Bytes> = shards[..self.config.data_shards]
            .iter()
            .map(|s| Bytes::from(s.clone()))
            .collect();

        let parity: Vec<Bytes> = shards[self.config.data_shards..]
            .iter()
            .map(|s| Bytes::from(s.clone()))
            .collect();

        let info = FecBlockInfo {
            block_id: self.block_id,
            algorithm: FecAlgorithm::ReedSolomon,
            data_shards: self.config.data_shards,
            parity_shards: self.config.parity_shards,
            shard_size,
            total_size: data.len(),
        };

        Ok((encoded_data, parity, info))
    }
}

/// FEC block information
#[derive(Debug, Clone)]
pub struct FecBlockInfo {
    pub block_id: u64,
    pub algorithm: FecAlgorithm,
    pub data_shards: usize,
    pub parity_shards: usize,
    pub shard_size: usize,
    pub total_size: usize,
}

/// Enhanced FEC decoder
pub struct EnhancedFecDecoder {
    algorithm: FecAlgorithm,
    config: FecConfig,
    rs_decoder: Option<ReedSolomon<Field>>,
    blocks: Arc<RwLock<HashMap<u64, FecBlockState>>>,
}

/// FEC block decoding state
struct FecBlockState {
    info: FecBlockInfo,
    received_shards: HashMap<usize, Bytes>,
    received_count: usize,
    decoded: Option<Bytes>,
}

impl EnhancedFecDecoder {
    /// Create new FEC decoder
    pub fn new(algorithm: FecAlgorithm, config: FecConfig) -> Result<Self> {
        let rs_decoder = match algorithm {
            FecAlgorithm::ReedSolomon => {
                Some(ReedSolomon::<Field>::new(config.data_shards, config.parity_shards)?)
            }
            FecAlgorithm::Xor => None,
        };

        Ok(Self {
            algorithm,
            config,
            rs_decoder,
            blocks: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Add a received shard
    pub fn add_shard(
        &self,
        block_id: u64,
        shard_index: usize,
        shard_data: Bytes,
        info: FecBlockInfo,
    ) -> Result<bool> {
        let mut blocks = self.blocks.write();
        
        let block = blocks.entry(block_id).or_insert_with(|| FecBlockState {
            info: info.clone(),
            received_shards: HashMap::new(),
            received_count: 0,
            decoded: None,
        });

        if block.received_shards.contains_key(&shard_index) {
            return Ok(false); // Already have this shard
        }

        block.received_shards.insert(shard_index, shard_data);
        block.received_count += 1;

        // Check if we can decode
        let total_needed = block.info.data_shards;
        Ok(block.received_count >= total_needed)
    }

    /// Attempt to decode a block
    pub fn decode(&self, block_id: u64) -> Result<Option<Bytes>> {
        let mut blocks = self.blocks.write();
        
        let block = blocks.get_mut(&block_id)
            .ok_or_else(|| anyhow::anyhow!("Block not found"))?;

        if let Some(decoded) = &block.decoded {
            return Ok(Some(decoded.clone()));
        }

        if block.received_count < block.info.data_shards {
            return Ok(None); // Not enough shards
        }

        let decoded = match self.algorithm {
            FecAlgorithm::Xor => self.decode_xor(block)?,
            FecAlgorithm::ReedSolomon => self.decode_reed_solomon(block)?,
        };

        block.decoded = Some(decoded.clone());
        Ok(Some(decoded))
    }

    /// XOR-based decoding
    fn decode_xor(&self, block: &FecBlockState) -> Result<Bytes> {
        let shard_size = block.info.shard_size;
        let mut reconstructed = BytesMut::with_capacity(block.info.total_size);

        // Reconstruct data shards
        for i in 0..block.info.data_shards {
            if let Some(shard) = block.received_shards.get(&i) {
                // Direct copy
                let end = (i + 1) * shard_size;
                let actual_end = end.min(block.info.total_size);
                let start = i * shard_size;
                
                if start < block.info.total_size {
                    reconstructed.extend_from_slice(&shard[..(actual_end - start).min(shard.len())]);
                }
            } else {
                // Need to reconstruct from parity
                // XOR all other shards with parity
                let mut shard = BytesMut::with_capacity(shard_size);
                shard.resize(shard_size, 0);

                // Use parity shard (index >= data_shards)
                for (idx, data) in &block.received_shards {
                    if *idx >= block.info.data_shards {
                        // This is a parity shard
                        for (j, &byte) in data.iter().enumerate() {
                            if j < shard.len() {
                                shard[j] ^= byte;
                            }
                        }
                    }
                }

                // XOR with all other data shards
                for (idx, data) in &block.received_shards {
                    if *idx < block.info.data_shards && *idx != i {
                        for (j, &byte) in data.iter().enumerate() {
                            if j < shard.len() {
                                shard[j] ^= byte;
                            }
                        }
                    }
                }

                let start = i * shard_size;
                let end = (start + shard_size).min(block.info.total_size);
                if start < block.info.total_size {
                    reconstructed.extend_from_slice(&shard[..(end - start).min(shard.len())]);
                }
            }
        }

        // Trim to actual size
        reconstructed.truncate(block.info.total_size);
        Ok(reconstructed.freeze())
    }

    /// Reed-Solomon decoding
    fn decode_reed_solomon(&self, block: &FecBlockState) -> Result<Bytes> {
        let decoder = self.rs_decoder.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Reed-Solomon decoder not initialized"))?;

        let shard_size = block.info.shard_size;
        let total_shards = block.info.data_shards + block.info.parity_shards;

        // Build shard array (None for missing shards)
        let mut shards: Vec<Option<Vec<u8>>> = (0..total_shards)
            .map(|i| {
                block.received_shards
                    .get(&i)
                    .map(|s| s.to_vec())
            })
            .collect();

        // Decode
        decoder.reconstruct(&mut shards)
            .context("Reed-Solomon reconstruction failed")?;

        // Reconstruct original data
        let mut reconstructed = BytesMut::with_capacity(block.info.total_size);
        for i in 0..block.info.data_shards {
            if let Some(Some(shard)) = shards.get(i) {
                let start = i * shard_size;
                let end = (start + shard_size).min(block.info.total_size);
                if start < block.info.total_size {
                    reconstructed.extend_from_slice(&shard[..(end - start).min(shard.len())]);
                }
            }
        }

        reconstructed.truncate(block.info.total_size);
        Ok(reconstructed.freeze())
    }

    /// Get FEC statistics
    pub fn stats(&self) -> FecStats {
        let blocks = self.blocks.read();
        
        let mut total_blocks = 0;
        let mut repaired_blocks = 0;
        let mut failed_blocks = 0;
        let mut total_shards_received = 0;
        let mut total_shards_needed = 0;

        for block in blocks.values() {
            total_blocks += 1;
            total_shards_received += block.received_count;
            total_shards_needed += block.info.data_shards;

            if block.decoded.is_some() {
                if block.received_count > block.info.data_shards {
                    repaired_blocks += 1;
                }
            } else if block.received_count < block.info.data_shards {
                failed_blocks += 1;
            }
        }

        FecStats {
            total_blocks,
            repaired_blocks,
            failed_blocks,
            repair_rate: if total_blocks > 0 {
                repaired_blocks as f32 / total_blocks as f32
            } else {
                0.0
            },
            parity_generation_rate: if total_shards_needed > 0 {
                (total_shards_received - total_shards_needed) as f32 / total_shards_needed as f32
            } else {
                0.0
            },
        }
    }

    /// Clean up old blocks
    pub fn cleanup_old_blocks(&self, max_age_seconds: u64) {
        // Implementation would remove blocks older than max_age_seconds
        // For now, just limit total blocks
        let mut blocks = self.blocks.write();
        if blocks.len() > 1000 {
            // Remove oldest 100 blocks
            let block_ids: Vec<u64> = blocks.keys().copied().take(100).collect();
            for id in block_ids {
                blocks.remove(&id);
            }
        }
    }
}

/// FEC statistics
#[derive(Debug, Clone)]
pub struct FecStats {
    pub total_blocks: usize,
    pub repaired_blocks: usize,
    pub failed_blocks: usize,
    pub repair_rate: f32,
    pub parity_generation_rate: f32,
}

