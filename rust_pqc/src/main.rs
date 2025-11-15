use std::path::PathBuf;
use std::fs::File;
use std::io::{Read, Write, BufReader, BufWriter};

use clap::{Parser, Subcommand};
use anyhow::Result;

use sha2::Sha256;
use hkdf::Hkdf;

use chacha20poly1305::{XChaCha20Poly1305, Key, XNonce, aead::Aead};
use chacha20poly1305::KeyInit;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

// pqcrypto-kyber provides kyber768 implementations
use pqcrypto_kyber::kyber768;
use pqcrypto_traits::kem::*;

const MAGIC: &[u8] = b"RKPQ1"; // header magic
const CHUNK_SIZE: usize = 1024 * 1024; // 1 MiB chunks

#[derive(Parser)]
#[command(author, version, about = "Rust PQC hybrid file encryptor (Kyber-768 + XChaCha20-Poly1305)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a Kyber-768 keypair
    Keygen {
        /// Output directory for keys
        #[arg(short, long, default_value = "keys")]
        outdir: PathBuf,
    },
    /// Encrypt a file for recipient public key
    Encrypt {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Recipient public key file (raw bytes)
        #[arg(short='p', long)]
        pubkey: PathBuf,
    },
    /// Decrypt a file with a Kyber private key
    Decrypt {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Private key file
        #[arg(short='k', long)]
        privkey: PathBuf,
    },
    /// Benchmark: run one KEM encapsulate and then do N in-memory encrypt/decrypt iterations
    BenchmarkSession {
        /// Recipient public key file
        #[arg(short='p', long)]
        pubkey: PathBuf,
        /// Number of iterations for encrypt/decrypt
        #[arg(short='n', long, default_value_t = 1000)]
        iterations: usize,
        /// Message size in bytes
        #[arg(short='s', long, default_value_t = 256)]
        size: usize,
    },
}

fn write_all<P: AsRef<std::path::Path>>(path: P, data: &[u8]) -> Result<()> {
    let mut f = File::create(path)?;
    f.write_all(data)?;
    Ok(())
}

fn read_all<P: AsRef<std::path::Path>>(path: P) -> Result<Vec<u8>> {
    let mut f = File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    Ok(buf)
}

fn keygen(outdir: PathBuf) -> Result<()> {
    std::fs::create_dir_all(&outdir)?;

    let (pk, sk) = kyber768::keypair();

    let pk_bytes = pk.as_bytes();
    let sk_bytes = sk.as_bytes();

    write_all(outdir.join("kyber_public.key"), pk_bytes)?;
    write_all(outdir.join("kyber_private.key"), sk_bytes)?;

    println!("Wrote kyber_public.key ({} bytes) and kyber_private.key ({} bytes)", pk_bytes.len(), sk_bytes.len());
    Ok(())
}

fn hkdf_derive(shared: &[u8], info: &[u8], out_len: usize) -> Result<Vec<u8>> {
    let hk = Hkdf::<Sha256>::new(None, shared);
    let mut okm = vec![0u8; out_len];
    hk.expand(info, &mut okm).map_err(|e| anyhow::anyhow!("hkdf expand failed: {:?}", e))?;
    Ok(okm)
}

fn encrypt_file(input: PathBuf, output: PathBuf, pubkey_path: PathBuf) -> Result<()> {
    // record wall-clock start and high-precision Instant
    let start_instant = Instant::now();
    let start_ts = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| anyhow::anyhow!("time error: {}", e))?.as_millis();
    println!("Encryption started: {} ms since epoch", start_ts);

    let pk_bytes = read_all(pubkey_path)?;
    let pk = kyber768::PublicKey::from_bytes(&pk_bytes).map_err(|e| anyhow::anyhow!("PublicKey from_bytes: {}", e))?;

    // Encapsulate to recipient public key -> (ciphertext, shared_secret)
    let (ct, shared) = kyber768::encapsulate(&pk);

    // Encapsulate returns (SharedSecret, Ciphertext) -> assign names accordingly
    let (shared, ct) = kyber768::encapsulate(&pk);

    // File key (32 bytes) used as AEAD key material
    let mut file_key = [0u8; 32];
    getrandom::getrandom(&mut file_key)?;

    // Derive KEK from shared secret
    let kek = hkdf_derive(shared.as_bytes(), b"kyber-kek-v1", 32)?;

    // Wrap file_key with KEK using XChaCha20-Poly1305
    let aead_kek = XChaCha20Poly1305::new(Key::from_slice(&kek));
    let mut wrap_nonce = [0u8; 24];
    getrandom::getrandom(&mut wrap_nonce)?;
    let wrap_ct = aead_kek.encrypt(XNonce::from_slice(&wrap_nonce), &file_key[..]).map_err(|e| anyhow::anyhow!("AEAD wrap error: {}", e))?;

    // Prepare output stream (buffered)
    let out_file = File::create(&output)?;
    let mut out = BufWriter::with_capacity(64 * 1024, out_file);
    // header: MAGIC
    out.write_all(MAGIC)?;

    // write kem ciphertext length + bytes (u16)
    let ct_bytes = ct.as_bytes();
    let ct_len = ct_bytes.len() as u16;
    out.write_all(&ct_len.to_be_bytes())?;
    out.write_all(ct_bytes)?;

    // write wrap nonce (24) and wrapped file-key length (u16) and wrapped bytes
    out.write_all(&wrap_nonce)?;
    let wrap_len = wrap_ct.len() as u16;
    out.write_all(&wrap_len.to_be_bytes())?;
    out.write_all(&wrap_ct)?;

    // Now stream file in chunks, for each chunk generate random nonce and encrypt
    let mut infile = BufReader::with_capacity(CHUNK_SIZE, File::open(&input)?);
    let mut buf = vec![0u8; CHUNK_SIZE];
    // reuse AEAD instance for file chunks
    let aead_file = XChaCha20Poly1305::new(Key::from_slice(&file_key));
    loop {
        let n = infile.read(&mut buf)?;
        if n == 0 { break; }
        let chunk = &buf[..n];

        let mut chunk_nonce = [0u8; 24];
        getrandom::getrandom(&mut chunk_nonce)?;
        let ct_chunk = aead_file.encrypt(XNonce::from_slice(&chunk_nonce), chunk).map_err(|e| anyhow::anyhow!("AEAD chunk encrypt: {}", e))?;

        // write nonce (24) + chunk ct len (u32 BE) + bytes
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

fn decrypt_file(input: PathBuf, output: PathBuf, privkey_path: PathBuf) -> Result<()> {
    let in_bytes = read_all(&input)?;
    let mut cursor = std::io::Cursor::new(&in_bytes);

    let mut magic = [0u8; 5];
    cursor.read_exact(&mut magic)?;
    if &magic != MAGIC { anyhow::bail!("invalid file format"); }

    // read kem ciphertext
    let mut ct_len_b = [0u8; 2];
    cursor.read_exact(&mut ct_len_b)?;
    let ct_len = u16::from_be_bytes(ct_len_b) as usize;
    let mut kem_ct = vec![0u8; ct_len];
    cursor.read_exact(&mut kem_ct)?;

    // read wrap nonce and wrapped key
    let mut wrap_nonce = [0u8; 24];
    cursor.read_exact(&mut wrap_nonce)?;
    let mut wrap_len_b = [0u8; 2];
    cursor.read_exact(&mut wrap_len_b)?;
    let wrap_len = u16::from_be_bytes(wrap_len_b) as usize;
    let mut wrap_ct = vec![0u8; wrap_len];
    cursor.read_exact(&mut wrap_ct)?;

    // decapsulate to obtain shared secret
    let sk_bytes = read_all(privkey_path)?;
    let sk = kyber768::SecretKey::from_bytes(&sk_bytes).map_err(|e| anyhow::anyhow!("SecretKey from_bytes: {}", e))?;
    // convert kem_ct bytes into typed Ciphertext
    let kem_ct_obj = kyber768::Ciphertext::from_bytes(&kem_ct).map_err(|e| anyhow::anyhow!("Ciphertext from_bytes: {}", e))?;
    let shared = kyber768::decapsulate(&kem_ct_obj, &sk);

    // derive KEK and unwrap file key
    let kek = hkdf_derive(shared.as_bytes(), b"kyber-kek-v1", 32)?;
    let aead_kek = XChaCha20Poly1305::new(Key::from_slice(&kek));
    let file_key = aead_kek.decrypt(XNonce::from_slice(&wrap_nonce), wrap_ct.as_ref()).map_err(|e| anyhow::anyhow!("AEAD unwrap error: {}", e))?;

    let out_file = File::create(output)?;
    let mut out = BufWriter::with_capacity(64 * 1024, out_file);

    // Now read chunks until cursor exhausted
    while (cursor.position() as usize) < in_bytes.len() {
        let mut chunk_nonce = [0u8; 24];
        cursor.read_exact(&mut chunk_nonce)?;
        let mut cl_b = [0u8; 4];
        cursor.read_exact(&mut cl_b)?;
        let cl = u32::from_be_bytes(cl_b) as usize;
        let mut ct_chunk = vec![0u8; cl];
        cursor.read_exact(&mut ct_chunk)?;
        // reuse AEAD instance
        let aead_file = XChaCha20Poly1305::new(Key::from_slice(&file_key));
        let pt = aead_file.decrypt(XNonce::from_slice(&chunk_nonce), ct_chunk.as_ref()).map_err(|e| anyhow::anyhow!("AEAD chunk decrypt: {}", e))?;
        out.write_all(&pt)?;
    }

    out.flush()?;
    println!("Decryption complete");
    Ok(())
}

fn benchmark_session(pubkey_path: PathBuf, iterations: usize, size: usize) -> Result<()> {
    // Read public key and encapsulate once
    let pk_bytes = read_all(pubkey_path)?;
    let pk = kyber768::PublicKey::from_bytes(&pk_bytes).map_err(|e| anyhow::anyhow!("PublicKey from_bytes: {}", e))?;
    let (shared, _ct) = kyber768::encapsulate(&pk);
    let session_key = hkdf_derive(shared.as_bytes(), b"kyber-session-v1", 32)?;

    // Prepare AEAD with session key
    let aead = XChaCha20Poly1305::new(Key::from_slice(&session_key));

    // prepare message
    let mut msg = vec![0u8; size];
    getrandom::getrandom(&mut msg)?;

    // warm up
    for _ in 0..10 {
        let mut nonce = [0u8; 24]; getrandom::getrandom(&mut nonce)?;
        let _ = aead.encrypt(XNonce::from_slice(&nonce), msg.as_ref()).map_err(|e| anyhow::anyhow!("warmup encrypt: {}", e))?;
    }

    // measure encrypt
    let mut enc_total_ns: u128 = 0;
    for _ in 0..iterations {
        let mut nonce = [0u8; 24]; getrandom::getrandom(&mut nonce)?;
        let t0 = std::time::Instant::now();
        let ct = aead.encrypt(XNonce::from_slice(&nonce), msg.as_ref()).map_err(|e| anyhow::anyhow!("encrypt: {}", e))?;
        enc_total_ns += t0.elapsed().as_nanos();
        // also decrypt once to ensure round-trip cost measured separately
        let t1 = std::time::Instant::now();
        let _pt = aead.decrypt(XNonce::from_slice(&nonce), ct.as_ref()).map_err(|e| anyhow::anyhow!("decrypt: {}", e))?;
        enc_total_ns += t1.elapsed().as_nanos();
    }

    let avg_ns = enc_total_ns as f64 / (iterations as f64 * 2.0);
    let avg_ms = avg_ns / 1_000_000.0;
    println!("Benchmark session: iterations={} size={} bytes -> avg per-op = {avg_ms:.6} ms ({avg_ns:.0} ns)", iterations, size);
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Keygen { outdir } => keygen(outdir)?,
        Commands::Encrypt { input, output, pubkey } => encrypt_file(input, output, pubkey)?,
        Commands::Decrypt { input, output, privkey } => decrypt_file(input, output, privkey)?,
        Commands::BenchmarkSession { pubkey, iterations, size } => benchmark_session(pubkey, iterations, size)?,
    }
    Ok(())
}
