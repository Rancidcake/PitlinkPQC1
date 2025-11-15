use std::fs::File;
use std::io::{Read, Write};
use anyhow::Result;
use sha2::Sha256;
use blake3;
use hkdf::Hkdf;

pub const CHUNK_SIZE: usize = 1024 * 1024; // 1 MiB
pub const MAGIC: &[u8] = b"RKPQ1";

pub fn write_all<P: AsRef<std::path::Path>>(path: P, data: &[u8]) -> Result<()> {
    let mut f = File::create(path)?;
    f.write_all(data)?;
    Ok(())
}

pub fn read_all<P: AsRef<std::path::Path>>(path: P) -> Result<Vec<u8>> {
    let mut f = File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn hkdf_derive(shared: &[u8], info: &[u8], out_len: usize) -> Result<Vec<u8>> {
    let hk = Hkdf::<Sha256>::new(None, shared);
    let mut okm = vec![0u8; out_len];
    hk.expand(info, &mut okm).map_err(|e| anyhow::anyhow!("hkdf expand failed: {:?}", e))?;
    Ok(okm)
}

/// Compute Blake3 hash of data
/// Blake3 is faster and more secure than SHA256 for most use cases
pub fn blake3_hash(data: &[u8]) -> [u8; 32] {
    blake3::hash(data).into()
}

/// Compute Blake3 hash with a key (for MAC/authentication)
pub fn blake3_keyed_hash(key: &[u8; 32], data: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_keyed(key);
    hasher.update(data);
    hasher.finalize().into()
}

/// Compute Blake3 hash and return as hex string
pub fn blake3_hash_hex(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

/// Derive key using Blake3 (for key derivation)
/// Note: context must be a valid UTF-8 string
pub fn blake3_derive_key(context: &str, input_key_material: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(input_key_material);
    hasher.finalize().into()
}
