use std::path::PathBuf;
use std::fs::File;
use std::io::{Read, Write, BufReader, BufWriter};

use anyhow::Result;
use chacha20poly1305::{XChaCha20Poly1305, Key, XNonce, aead::Aead};
use chacha20poly1305::KeyInit;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use pqcrypto_kyber::kyber768;
use pqcrypto_traits::kem::*;
use getrandom;

use common::{read_all, write_all, hkdf_derive, CHUNK_SIZE, MAGIC};

/// Generate Kyber-768 keypair
pub fn keygen(outdir: PathBuf) -> Result<()> {
    std::fs::create_dir_all(&outdir)?;

    let (pk, sk) = kyber768::keypair();

    let pk_bytes = pk.as_bytes();
    let sk_bytes = sk.as_bytes();

    write_all(outdir.join("kyber_public.key"), pk_bytes)?;
    write_all(outdir.join("kyber_private.key"), sk_bytes)?;

    println!("Wrote kyber_public.key ({} bytes) and kyber_private.key ({} bytes)", pk_bytes.len(), sk_bytes.len());
    Ok(())
}

/// Encrypt a file using Kyber-768 + XChaCha20-Poly1305
pub fn encrypt_file(input: PathBuf, output: PathBuf, pubkey_path: PathBuf) -> Result<()> {
    let start_instant = Instant::now();
    let start_ts = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| anyhow::anyhow!("time error: {}", e))?.as_millis();
    println!("Encryption started: {} ms since epoch", start_ts);

    let pk_bytes = read_all(pubkey_path)?;
    let pk = kyber768::PublicKey::from_bytes(&pk_bytes).map_err(|e| anyhow::anyhow!("PublicKey from_bytes: {}", e))?;

    // encapsulate
    let (shared, ct) = kyber768::encapsulate(&pk);

    // File key (32 bytes)
    let mut file_key = [0u8; 32];
    getrandom::getrandom(&mut file_key)?;

    let kek = hkdf_derive(shared.as_bytes(), b"kyber-kek-v1", 32)?;

    let aead_kek = XChaCha20Poly1305::new(Key::from_slice(&kek));
    let mut wrap_nonce = [0u8; 24];
    getrandom::getrandom(&mut wrap_nonce)?;
    let wrap_ct = aead_kek.encrypt(XNonce::from_slice(&wrap_nonce), &file_key[..]).map_err(|e| anyhow::anyhow!("AEAD wrap error: {}", e))?;

    let out_file = File::create(&output)?;
    let mut out = BufWriter::with_capacity(64 * 1024, out_file);
    out.write_all(MAGIC)?;

    let ct_bytes = ct.as_bytes();
    let ct_len = ct_bytes.len() as u16;
    out.write_all(&ct_len.to_be_bytes())?;
    out.write_all(ct_bytes)?;

    out.write_all(&wrap_nonce)?;
    let wrap_len = wrap_ct.len() as u16;
    out.write_all(&wrap_len.to_be_bytes())?;
    out.write_all(&wrap_ct)?;

    let mut infile = BufReader::with_capacity(CHUNK_SIZE, File::open(&input)?);
    let mut buf = vec![0u8; CHUNK_SIZE];
    let aead_file = XChaCha20Poly1305::new(Key::from_slice(&file_key));
    loop {
        let n = infile.read(&mut buf)?;
        if n == 0 { break; }
        let chunk = &buf[..n];
        let mut chunk_nonce = [0u8; 24];
        getrandom::getrandom(&mut chunk_nonce)?;
        let ct_chunk = aead_file.encrypt(XNonce::from_slice(&chunk_nonce), chunk).map_err(|e| anyhow::anyhow!("AEAD chunk encrypt: {}", e))?;
        out.write_all(&chunk_nonce)?;
        let cl = ct_chunk.len() as u32;
        out.write_all(&cl.to_be_bytes())?;
        out.write_all(&ct_chunk)?;
    }
    out.flush()?;

    println!("Wrote encrypted package to {}", output.display());
    let end_ts = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| anyhow::anyhow!("time error: {}", e))?.as_millis();
    let elapsed = start_instant.elapsed();
    let elapsed_ms = (elapsed.as_secs() as u128) * 1000u128 + (elapsed.subsec_micros() as u128) / 1000u128;
    println!("Encryption finished: {} ms since epoch", end_ts);
    println!("Encryption elapsed: {} ms ({} us)", elapsed_ms, elapsed.as_micros());
    Ok(())
}

/// Decrypt a file using Kyber-768 + XChaCha20-Poly1305
pub fn decrypt_file(input: PathBuf, output: PathBuf, privkey_path: PathBuf) -> Result<()> {
    let in_bytes = read_all(&input)?;
    let mut cursor = std::io::Cursor::new(&in_bytes);

    let mut magic = [0u8; 5];
    cursor.read_exact(&mut magic)?;
    if &magic != MAGIC { anyhow::bail!("invalid file format"); }

    let mut ct_len_b = [0u8; 2];
    cursor.read_exact(&mut ct_len_b)?;
    let ct_len = u16::from_be_bytes(ct_len_b) as usize;
    let mut kem_ct = vec![0u8; ct_len];
    cursor.read_exact(&mut kem_ct)?;

    let mut wrap_nonce = [0u8; 24];
    cursor.read_exact(&mut wrap_nonce)?;
    let mut wrap_len_b = [0u8; 2];
    cursor.read_exact(&mut wrap_len_b)?;
    let wrap_len = u16::from_be_bytes(wrap_len_b) as usize;
    let mut wrap_ct = vec![0u8; wrap_len];
    cursor.read_exact(&mut wrap_ct)?;

    let sk_bytes = read_all(privkey_path)?;
    let sk = kyber768::SecretKey::from_bytes(&sk_bytes).map_err(|e| anyhow::anyhow!("SecretKey from_bytes: {}", e))?;
    let kem_ct_obj = kyber768::Ciphertext::from_bytes(&kem_ct).map_err(|e| anyhow::anyhow!("Ciphertext from_bytes: {}", e))?;
    let shared = kyber768::decapsulate(&kem_ct_obj, &sk);

    let kek = hkdf_derive(shared.as_bytes(), b"kyber-kek-v1", 32)?;
    let aead_kek = XChaCha20Poly1305::new(Key::from_slice(&kek));
    let file_key = aead_kek.decrypt(XNonce::from_slice(&wrap_nonce), wrap_ct.as_ref()).map_err(|e| anyhow::anyhow!("AEAD unwrap error: {}", e))?;

    let out_file = File::create(output)?;
    let mut out = BufWriter::with_capacity(64 * 1024, out_file);

    while (cursor.position() as usize) < in_bytes.len() {
        let mut chunk_nonce = [0u8; 24];
        cursor.read_exact(&mut chunk_nonce)?;
        let mut cl_b = [0u8; 4];
        cursor.read_exact(&mut cl_b)?;
        let cl = u32::from_be_bytes(cl_b) as usize;
        let mut ct_chunk = vec![0u8; cl];
        cursor.read_exact(&mut ct_chunk)?;
        let aead_file = XChaCha20Poly1305::new(Key::from_slice(&file_key));
        let pt = aead_file.decrypt(XNonce::from_slice(&chunk_nonce), ct_chunk.as_ref()).map_err(|e| anyhow::anyhow!("AEAD chunk decrypt: {}", e))?;
        out.write_all(&pt)?;
    }

    out.flush()?;
    println!("Decryption complete");
    Ok(())
}

/// Benchmark encryption/decryption session
pub fn benchmark_session(pubkey_path: PathBuf, iterations: usize, size: usize) -> Result<()> {
    let pk_bytes = read_all(pubkey_path)?;
    let pk = kyber768::PublicKey::from_bytes(&pk_bytes).map_err(|e| anyhow::anyhow!("PublicKey from_bytes: {}", e))?;
    let (shared, _ct) = kyber768::encapsulate(&pk);
    let session_key = hkdf_derive(shared.as_bytes(), b"kyber-session-v1", 32)?;

    let aead = XChaCha20Poly1305::new(Key::from_slice(&session_key));
    let mut msg = vec![0u8; size];
    getrandom::getrandom(&mut msg)?;

    for _ in 0..10 {
        let mut nonce = [0u8; 24]; getrandom::getrandom(&mut nonce)?;
        let _ = aead.encrypt(XNonce::from_slice(&nonce), msg.as_ref()).map_err(|e| anyhow::anyhow!("warmup encrypt: {}", e))?;
    }

    let mut enc_total_ns: u128 = 0;
    for _ in 0..iterations {
        let mut nonce = [0u8; 24]; getrandom::getrandom(&mut nonce)?;
        let t0 = std::time::Instant::now();
        let ct = aead.encrypt(XNonce::from_slice(&nonce), msg.as_ref()).map_err(|e| anyhow::anyhow!("encrypt: {}", e))?;
        enc_total_ns += t0.elapsed().as_nanos();
        let t1 = std::time::Instant::now();
        let _pt = aead.decrypt(XNonce::from_slice(&nonce), ct.as_ref()).map_err(|e| anyhow::anyhow!("decrypt: {}", e))?;
        enc_total_ns += t1.elapsed().as_nanos();
    }

    let avg_ns = enc_total_ns as f64 / (iterations as f64 * 2.0);
    let avg_ms = avg_ns / 1_000_000.0;
    println!("Benchmark session: iterations={} size={} bytes -> avg per-op = {avg_ms:.6} ms ({avg_ns:.0} ns)", iterations, size);
    Ok(())
}
