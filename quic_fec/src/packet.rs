//! QUIC-FEC packet format
//! Optimized packet structure for telemetry transfer with FEC support

use bytes::{Bytes, BytesMut, Buf, BufMut};
use anyhow::Result;
use common::blake3_hash;

/// Packet type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    Data = 0,
    FecParity = 1,
    Handover = 2,
    Ack = 3,
    Heartbeat = 4,
}

impl From<u8> for PacketType {
    fn from(value: u8) -> Self {
        match value {
            0 => PacketType::Data,
            1 => PacketType::FecParity,
            2 => PacketType::Handover,
            3 => PacketType::Ack,
            4 => PacketType::Heartbeat,
            _ => PacketType::Data,
        }
    }
}

/// Packet header (16 bytes)
#[derive(Debug, Clone)]
pub struct PacketHeader {
    /// Packet type
    pub packet_type: PacketType,
    /// Sequence number
    pub sequence: u64,
    /// FEC block ID (for grouping shards)
    pub fec_block_id: u32,
    /// Shard index within FEC block (0 for data shards, data_shards+ for parity)
    pub shard_index: u16,
    /// Total shards in FEC block
    pub total_shards: u16,
    /// Data length
    pub data_len: u16,
    /// Blake3 hash of packet data (first 16 bytes of 32-byte hash)
    pub checksum: [u8; 16],
}

impl PacketHeader {
    /// Size of header in bytes
    pub const SIZE: usize = 16;

    /// Serialize header to bytes
    pub fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(Self::SIZE);
        buf.put_u8(self.packet_type as u8);
        buf.put_u64(self.sequence);
        buf.put_u32(self.fec_block_id);
        buf.put_u16(self.shard_index);
        buf.put_u16(self.total_shards);
        buf.put_u16(self.data_len);
        buf.put_slice(&self.checksum);
        buf.freeze()
    }

    /// Deserialize header from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            anyhow::bail!("Header too short: {} bytes", data.len());
        }

        let mut buf = &data[..];
        let packet_type = PacketType::from(buf.get_u8());
        let sequence = buf.get_u64();
        let fec_block_id = buf.get_u32();
        let shard_index = buf.get_u16();
        let total_shards = buf.get_u16();
        let data_len = buf.get_u16();
        let mut checksum = [0u8; 16];
        buf.copy_to_slice(&mut checksum);

        Ok(Self {
            packet_type,
            sequence,
            fec_block_id,
            shard_index,
            total_shards,
            data_len,
            checksum,
        })
    }
}

/// Complete QUIC-FEC packet
#[derive(Debug, Clone)]
pub struct QuicFecPacket {
    pub header: PacketHeader,
    pub data: Bytes,
}

impl QuicFecPacket {
    /// Create a new data packet
    pub fn new_data(
        sequence: u64,
        fec_block_id: u32,
        shard_index: u16,
        total_shards: u16,
        data: Bytes,
    ) -> Self {
        let checksum = {
            let hash = blake3_hash(&data);
            let mut checksum = [0u8; 16];
            checksum.copy_from_slice(&hash[..16]);
            checksum
        };

        let header = PacketHeader {
            packet_type: PacketType::Data,
            sequence,
            fec_block_id,
            shard_index,
            total_shards,
            data_len: data.len() as u16,
            checksum,
        };

        Self { header, data }
    }

    /// Create a new FEC parity packet
    pub fn new_fec_parity(
        sequence: u64,
        fec_block_id: u32,
        shard_index: u16,
        total_shards: u16,
        data: Bytes,
    ) -> Self {
        let checksum = {
            let hash = blake3_hash(&data);
            let mut checksum = [0u8; 16];
            checksum.copy_from_slice(&hash[..16]);
            checksum
        };

        let header = PacketHeader {
            packet_type: PacketType::FecParity,
            sequence,
            fec_block_id,
            shard_index,
            total_shards,
            data_len: data.len() as u16,
            checksum,
        };

        Self { header, data }
    }

    /// Create a handover packet (for network path switching)
    pub fn new_handover(
        sequence: u64,
        new_path_info: &[u8],
    ) -> Self {
        let data = Bytes::copy_from_slice(new_path_info);
        let checksum = {
            let hash = blake3_hash(&data);
            let mut checksum = [0u8; 16];
            checksum.copy_from_slice(&hash[..16]);
            checksum
        };

        let header = PacketHeader {
            packet_type: PacketType::Handover,
            sequence,
            fec_block_id: 0,
            shard_index: 0,
            total_shards: 1,
            data_len: data.len() as u16,
            checksum,
        };

        Self { header, data }
    }

    /// Serialize packet to bytes
    pub fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(PacketHeader::SIZE + self.data.len());
        buf.put_slice(&self.header.to_bytes());
        buf.put_slice(&self.data);
        buf.freeze()
    }

    /// Deserialize packet from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < PacketHeader::SIZE {
            anyhow::bail!("Packet too short: {} bytes", data.len());
        }

        let header = PacketHeader::from_bytes(&data[..PacketHeader::SIZE])?;
        let payload = Bytes::copy_from_slice(&data[PacketHeader::SIZE..]);

        // Verify checksum
        let expected_hash = blake3_hash(&payload);
        if expected_hash[..16] != header.checksum {
            anyhow::bail!("Checksum mismatch");
        }

        Ok(Self {
            header,
            data: payload,
        })
    }

    /// Verify packet integrity
    pub fn verify(&self) -> bool {
        let hash = blake3_hash(&self.data);
        hash[..16] == self.header.checksum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_serialization() {
        let data = Bytes::from("Hello, World!");
        let packet = QuicFecPacket::new_data(1, 100, 0, 4, data.clone());

        let serialized = packet.to_bytes();
        let deserialized = QuicFecPacket::from_bytes(&serialized).unwrap();

        assert_eq!(deserialized.header.sequence, 1);
        assert_eq!(deserialized.header.fec_block_id, 100);
        assert_eq!(deserialized.data, data);
        assert!(deserialized.verify());
    }
}

