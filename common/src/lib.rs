use std::fs::File;
use std::io::{Read, Write};
use anyhow::Result;
use sha2::Sha256;
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
