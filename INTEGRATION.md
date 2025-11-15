# PitlinkPQC - Complete System Integration Guide

## Overview

This project combines multiple components into a unified telemetry and file transfer system:

1. **Brain (trackshift)**: AI-powered telemetry decision engine
2. **rust_pqc**: Post-quantum cryptography encryption/decryption
3. **lz4_chunker**: LZ4 compression for efficient data transfer
4. **csv_lz4_tool**: CSV-specific compression utilities
5. **common**: Shared utilities and helpers

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│              PitlinkPQC Integrated System                   │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │         Telemetry Processing Pipeline                │  │
│  │                                                       │  │
│  │  [Telemetry Data] → [AI Analysis] → [Decision]      │  │
│  │         ↓                    ↓           ↓           │  │
│  │  [Compression]      [Encryption]   [Routing]         │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │         File Transfer Pipeline                        │  │
│  │                                                       │  │
│  │  [File] → [Chunk] → [Encrypt] → [Compress] → [Send] │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Component Integration

### 1. Brain (AI Telemetry Engine)

**Location**: `brain/`

**Purpose**: Makes intelligent decisions about:
- Network routing (5G/WiFi/Starlink/Multipath)
- Data compression needs
- Redundancy detection
- Network quality assessment
- Transfer optimization

**Key Features**:
- ONNX model inference for decision making
- Vector similarity search for context
- Adaptive thresholds based on network quality
- Redundancy detection (saves 30-80% bandwidth)

**Usage**:
```rust
use trackshift::telemetry_ai::*;

let ai = TelemetryAi::new("models/slm.onnx", "models/embedder.onnx")?;
let decision = ai.process_chunk(chunk_data, network_metrics)?;
```

### 2. Rust PQC (Post-Quantum Encryption)

**Location**: `rust_pqc/`

**Purpose**: Provides post-quantum secure encryption using:
- Kyber-768 KEM (Key Encapsulation Mechanism)
- XChaCha20-Poly1305 AEAD encryption

**Key Features**:
- Chunked encryption (1 MiB chunks)
- High throughput (175+ MB/s)
- Future-proof security

**Usage**:
```rust
use rust_pqc::*;

// Encrypt
encrypt_file(input_path, output_path, public_key_path)?;

// Decrypt
decrypt_file(input_path, output_path, private_key_path)?;
```

### 3. Compression (LZ4 & Zstd)

**Location**: `lz4_chunker/`, `csv_lz4_tool/`, `brain/src/integration.rs`

**Purpose**: Fast compression for efficient data transfer

**Key Features**:
- **LZ4**: Very fast compression/decompression, lower ratio
- **Zstd**: Balanced compression, better ratio
- **Auto-selection**: Intelligently chooses based on data and network
- Dynamic chunk sizing
- CSV-specific optimizations

**Compression Algorithms**:
- `CompressionAlgorithm::Lz4` - Fast, best for small files
- `CompressionAlgorithm::Zstd` - Balanced, best for large files
- `CompressionAlgorithm::Auto` - AI-driven selection

### 4. Integrated Pipeline

**Location**: `brain/src/integration.rs`

**Purpose**: Combines all components into a unified processing pipeline

**Usage**:
```rust
use trackshift::integration::*;

// Initialize pipeline
let pipeline = IntegratedTelemetryPipeline::new(
    "models/slm.onnx",
    "models/embedder.onnx",
    true,  // Enable encryption
    true,  // Enable compression
)?;

// Process chunk with full pipeline
let processed = pipeline.process_chunk_full(chunk_data, network_metrics)?;

match processed.action {
    ProcessAction::SendFull => { /* Send uncompressed */ }
    ProcessAction::SendCompressed => { /* Send compressed */ }
    ProcessAction::SendEncrypted => { /* Encrypt then send */ }
    ProcessAction::Buffer => { /* Buffer for later */ }
    ProcessAction::Skip => { /* Skip redundant data */ }
}
```

## Complete Workflow Example

### Scenario: Telemetry Data Transfer

```rust
use trackshift::integration::*;
use trackshift::telemetry_ai::*;

// 1. Initialize pipeline
let pipeline = IntegratedTelemetryPipeline::new(
    "models/slm.onnx",
    "models/embedder.onnx",
    true,  // encryption
    true,  // compression
)?;

// 2. Collect network metrics
let metrics = NetworkMetricsInput {
    rtt_ms: 20.0,
    jitter_ms: 3.0,
    loss_rate: 0.001,
    throughput_mbps: 100.0,
    wifi_signal: -50.0,
    fiveg_signal: -60.0,
    ..Default::default()
};

// 3. Process telemetry chunk
let telemetry_data = b"Temperature: 25.5C, Humidity: 60%";
let processed = pipeline.process_chunk_full(telemetry_data, metrics)?;

// 4. Handle based on decision
if processed.decision.should_buffer {
    // Network too poor - buffer for later
    buffer.add(processed.processed_data.unwrap())?;
} else if processed.decision.should_send {
    match processed.action {
        ProcessAction::SendEncrypted => {
            // Encrypt using rust_pqc
            encrypt_chunk(&processed.processed_data.unwrap(), pubkey)?;
        }
        ProcessAction::SendCompressed => {
            // Already compressed by pipeline
            send_chunk(&processed.processed_data.unwrap())?;
        }
        _ => {
            send_chunk(&processed.processed_data.unwrap())?;
        }
    }
} else {
    // Skip redundant data
    println!("Skipping redundant chunk (similarity: {:.2})", 
        processed.decision.similarity_score);
}
```

### Scenario: File Transfer with Encryption

```rust
use rust_pqc::*;
use std::path::PathBuf;

// 1. Generate keys (one-time setup)
keygen(PathBuf::from("keys/recipient"))?;

// 2. Encrypt file before transfer
encrypt_file(
    PathBuf::from("document.pdf"),
    PathBuf::from("document.pdf.rkpq"),
    PathBuf::from("keys/recipient/kyber_public.key"),
)?;

// 3. Transfer encrypted file through network

// 4. Decrypt after receiving
decrypt_file(
    PathBuf::from("received/document.pdf.rkpq"),
    PathBuf::from("decrypted/document.pdf"),
    PathBuf::from("keys/recipient/kyber_private.key"),
)?;
```

## Building the Complete System

### Prerequisites

1. Rust toolchain (latest stable)
2. ONNX models (for brain component):
   - `brain/models/slm.onnx`
   - `brain/models/embedder.onnx`

### Build All Components

```bash
# Build entire workspace
cargo build --release

# Build specific component
cargo build -p trackshift --release
cargo build -p rust_pqc --release
```

### Run Examples

```bash
# Run integrated workflow example
cargo run --example integrated_workflow -p trackshift

# Run patchy network example
cargo run --example patchy_network_example -p trackshift

# Run scheduler integration example
cargo run --example scheduler_integration -p trackshift
```

## Component Dependencies

```
brain (trackshift)
├── ort (ONNX Runtime)
├── ndarray
├── lz4_flex (compression)
└── parking_lot

rust_pqc
├── pqcrypto-kyber
├── chacha20poly1305
├── common (shared utilities)
└── tokio

lz4_chunker
└── lz4_flex

csv_lz4_tool
└── lz4_flex

common
├── sha2
├── hkdf
└── anyhow
```

## Integration Points

### 1. AI → Encryption

When AI detects sensitive data or poor network:
- Enable encryption using `rust_pqc`
- Use AI's `severity` and `network_quality` to determine encryption needs

### 2. AI → Compression

When AI recommends compression:
- Use `optimization_hint: OptimizationHint::Compress`
- Apply compression (LZ4 or Zstd) based on selection:
  - **Auto mode**: Selects Zstd for large files (>1MB) or poor networks
  - **Manual**: Use `with_compression_algorithm()` to specify
- Compression algorithm tracked in `ProcessedChunk.compression_algorithm`

### 3. AI → File Transfer

When transferring files:
- Use AI's `process_file_transfer()` method
- Get recommendations for chunk size, parallel streams, integrity checks
- Apply encryption/compression based on AI decisions

### 4. Network Quality → All Components

AI's network quality assessment affects:
- **Encryption**: More aggressive on patchy networks
- **Compression**: Always compress on poor networks
- **Buffering**: Buffer when network is down
- **Retry Strategy**: Exponential backoff on patchy networks

## Best Practices

1. **Initialize AI First**: Load ONNX models at startup
2. **Monitor Network Quality**: Use AI's network quality assessment
3. **Adaptive Thresholds**: Let AI adjust redundancy thresholds
4. **Encrypt Sensitive Data**: Use rust_pqc for critical information
5. **Compress Large Transfers**: Use LZ4 for bandwidth savings
6. **Handle Failures Gracefully**: Use AI's retry strategies

## Performance Characteristics

| Component | Latency | Throughput |
|-----------|---------|------------|
| AI Decision | < 3ms | N/A |
| Encryption | ~30ms setup | 175+ MB/s |
| Compression | < 1ms | 500+ MB/s |
| Integrated Pipeline | < 5ms | Limited by slowest component |

## Security Considerations

1. **Key Management**: Store private keys securely
2. **Model Security**: Protect ONNX models from tampering
3. **Network Security**: Use encryption for sensitive telemetry
4. **Integrity Checks**: Verify data after transfer

## Troubleshooting

### AI Models Not Found
- Ensure `brain/models/slm.onnx` and `brain/models/embedder.onnx` exist
- See `brain/SETUP.md` for model generation

### Encryption Fails
- Verify key files exist and are readable
- Check file permissions on key directories

### Compression Issues
- Ensure `lz4_flex` dependency is available
- Check data size (very small chunks may not compress well)

### Build Errors
- Run `cargo clean` and rebuild
- Check Rust version: `rustc --version` (should be 1.70+)

## Next Steps

1. **Deploy Models**: Set up ONNX models for your use case
2. **Configure Keys**: Generate encryption keypairs for recipients
3. **Integrate**: Use the integrated pipeline in your application
4. **Monitor**: Track performance and adjust thresholds
5. **Optimize**: Fine-tune AI models based on your data patterns

## Documentation

- **Brain Component**: See `brain/README.md`
- **Encryption**: See `rust_pqc/README.md`
- **Quick Start**: See `QUICKSTART.md`
- **Production**: See `PRODUCTION_INTEGRATION.md`

## Support

For issues or questions:
1. Check component-specific documentation
2. Review examples in `brain/examples/`
3. Check troubleshooting sections in component READMEs

