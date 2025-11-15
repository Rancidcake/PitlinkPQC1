use std::path::PathBuf;
use clap::{Parser, Subcommand};
use anyhow::Result;
use rust_pqc::{keygen, encrypt_file, decrypt_file, benchmark_session};

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
    /// Benchmark session mode
    BenchmarkSession {
        #[arg(short='p', long)]
        pubkey: PathBuf,
        #[arg(short='n', long, default_value_t = 1000)]
        iterations: usize,
        #[arg(short='s', long, default_value_t = 256)]
        size: usize,
    },
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
