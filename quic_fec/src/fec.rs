//! Forward Error Correction (FEC) using Reed-Solomon erasure coding
//! Optimized for QUIC handover scenarios where packets may be lost during network transitions

use anyhow::{Result, Context};
use reed_solomon_erasure::{ReedSolomon, galois_8::Field};
use bytes::{Bytes, BytesMut};
use std::sync::Arc;
use parking_lot::RwLock;

/// FEC configuration
#[derive(Debug, Clone)]
pub struct FecConfig {
    /// Number of data shards (original packets)
    pub data_shards: usize,
    /// Number of parity shards (redundancy packets)
    pub parity_shards: usize,
    /// Maximum shard size in bytes
    pub max_shard_size: usize,
}

impl Default for FecConfig {
    fn default() -> Self {
        Self {
            data_shards: 4,
            parity_shards: 2,  // Can recover from 2 lost packets
            max_shard_size: 1400,  // QUIC MTU
        }
    }
}

impl FecConfig {
    /// Create config optimized for telemetry transfer
    pub fn for_telemetry() -> Self {
        Self {
            data_shards: 8,
            parity_shards: 3,  // Can recover from 3 lost packets (good for handover)
            max_shard_size: 1200,
        }
    }

    /// Create config optimized for file transfer
    pub fn for_file_transfer() -> Self {
        Self {
            data_shards: 16,
            parity_shards: 4,  // Can recover from 4 lost packets
            max_shard_size: 1400,
        }
    }

    /// Create config for high-loss scenarios (patchy networks)
    pub fn for_patchy_network() -> Self {
        Self {
            data_shards: 4,
            parity_shards: 4,  // 50% redundancy - can recover from 4 lost packets
            max_shard_size: 1200,
        }
    }
}

/// FEC Encoder - encodes data into shards with parity
pub struct FecEncoder {
    rs: Arc<ReedSolomon<Field>>,
    config: FecConfig,
}

impl FecEncoder {
    /// Create a new FEC encoder
    pub fn new(config: FecConfig) -> Result<Self> {
        let rs = Arc::new(
            ReedSolomon::new(config.data_shards, config.parity_shards)
                .context("Failed to create Reed-Solomon encoder")?
        );

        Ok(Self {
            rs,
            config,
        })
    }

    /// Encode data into shards (data + parity)
    /// Returns (data_shards, parity_shards)
    pub fn encode(&self, data: &[u8]) -> Result<(Vec<Bytes>, Vec<Bytes>)> {
        // Split data into shards
        let shard_size = (data.len() + self.config.data_shards - 1) / self.config.data_shards;
        let total_shards = self.config.data_shards + self.config.parity_shards;
        
        // Pad data to fit shards
        let padded_size = shard_size * self.config.data_shards;
        let mut padded_data = data.to_vec();
        padded_data.resize(padded_size, 0);

        // Create shards
        let mut shards: Vec<Vec<u8>> = (0..total_shards)
            .map(|_| vec![0u8; shard_size])
            .collect();

        // Fill data shards
        for (i, chunk) in padded_data.chunks(shard_size).enumerate() {
            if i < self.config.data_shards {
                shards[i][..chunk.len()].copy_from_slice(chunk);
            }
        }

        // Encode parity
        self.rs.encode(&mut shards)
            .context("Failed to encode FEC shards")?;

        // Convert to Bytes
        let data_shards: Vec<Bytes> = shards[..self.config.data_shards]
            .iter()
            .map(|s| Bytes::copy_from_slice(s))
            .collect();

        let parity_shards: Vec<Bytes> = shards[self.config.data_shards..]
            .iter()
            .map(|s| Bytes::copy_from_slice(s))
            .collect();

        Ok((data_shards, parity_shards))
    }

    /// Get the total number of shards (data + parity)
    pub fn total_shards(&self) -> usize {
        self.config.data_shards + self.config.parity_shards
    }

    /// Get the number of parity shards
    pub fn parity_shards(&self) -> usize {
        self.config.parity_shards
    }
}

/// FEC Decoder - reconstructs data from received shards
pub struct FecDecoder {
    rs: Arc<ReedSolomon<Field>>,
    config: FecConfig,
    received_shards: Arc<RwLock<Vec<Option<Bytes>>>>,
    shard_size: Arc<RwLock<Option<usize>>>,
}

impl FecDecoder {
    /// Create a new FEC decoder
    pub fn new(config: FecConfig) -> Result<Self> {
        let rs = Arc::new(
            ReedSolomon::new(config.data_shards, config.parity_shards)
                .context("Failed to create Reed-Solomon decoder")?
        );

        let total_shards = config.data_shards + config.parity_shards;
        let received_shards = Arc::new(RwLock::new(vec![None; total_shards]));

        Ok(Self {
            rs,
            config,
            received_shards,
            shard_size: Arc::new(RwLock::new(None)),
        })
    }

    /// Add a received shard
    /// Returns true if we have enough shards to decode
    pub fn add_shard(&self, shard_index: usize, shard_data: Bytes) -> Result<bool> {
        let mut shards = self.received_shards.write();
        
        if shard_index >= shards.len() {
            anyhow::bail!("Shard index {} out of range (max {})", shard_index, shards.len() - 1);
        }

        // Set shard size on first shard
        {
            let mut size = self.shard_size.write();
            if size.is_none() {
                *size = Some(shard_data.len());
            }
        }

        shards[shard_index] = Some(shard_data);

        // Check if we have enough shards to decode
        let received_count = shards.iter().filter(|s| s.is_some()).count();
        Ok(received_count >= self.config.data_shards)
    }

    /// Try to decode the original data
    /// Returns None if not enough shards received
    pub fn decode(&self) -> Result<Option<Bytes>> {
        let shards = self.received_shards.read();
        let received_count = shards.iter().filter(|s| s.is_some()).count();

        if received_count < self.config.data_shards {
            return Ok(None);
        }

        let shard_size = self.shard_size.read()
            .ok_or_else(|| anyhow::anyhow!("Shard size not set"))?;

        // Convert to Vec<Option<Vec<u8>>>
        let mut shard_vecs: Vec<Option<Vec<u8>>> = shards
            .iter()
            .map(|s| s.as_ref().map(|b| b.to_vec()))
            .collect();

        // Reconstruct missing shards
        self.rs.reconstruct(&mut shard_vecs)
            .context("Failed to reconstruct FEC shards")?;

        // Extract data shards
        let data_shards: Vec<&[u8]> = shard_vecs[..self.config.data_shards]
            .iter()
            .map(|s| s.as_ref().unwrap().as_slice())
            .collect();

        // Concatenate data shards
        let total_size = shard_size * self.config.data_shards;
        let mut result = BytesMut::with_capacity(total_size);
        
        for shard in data_shards {
            result.extend_from_slice(shard);
        }

        // Remove padding (find first zero block)
        let data = result.freeze();
        Ok(Some(data))
    }

    /// Reset decoder for new block
    pub fn reset(&self) {
        let mut shards = self.received_shards.write();
        *shards = vec![None; self.config.data_shards + self.config.parity_shards];
        let mut size = self.shard_size.write();
        *size = None;
    }

    /// Get number of received shards
    pub fn received_count(&self) -> usize {
        let shards = self.received_shards.read();
        shards.iter().filter(|s| s.is_some()).count()
    }

    /// Get number of missing shards
    pub fn missing_count(&self) -> usize {
        let total = self.config.data_shards + self.config.parity_shards;
        total - self.received_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fec_encode_decode() {
        let config = FecConfig::default();
        let encoder = FecEncoder::new(config.clone()).unwrap();
        let decoder = FecDecoder::new(config).unwrap();

        let original_data = b"Hello, World! This is a test message for FEC encoding.";
        
        // Encode
        let (data_shards, parity_shards) = encoder.encode(original_data).unwrap();
        
        // Simulate packet loss - lose 2 data shards
        let mut received = Vec::new();
        for (i, shard) in data_shards.iter().enumerate() {
            if i < 2 {
                // Skip first 2 shards (simulate loss)
                continue;
            }
            decoder.add_shard(i, shard.clone()).unwrap();
        }
        
        // Add parity shards
        for (i, shard) in parity_shards.iter().enumerate() {
            decoder.add_shard(encoder.config.data_shards + i, shard.clone()).unwrap();
        }

        // Decode
        let decoded = decoder.decode().unwrap().unwrap();
        
        // Verify original data is recovered (may have padding)
        assert!(decoded.starts_with(original_data));
    }
}

