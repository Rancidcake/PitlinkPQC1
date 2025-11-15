# Zstd Compression Integration

## Overview

Zstd (Zstandard) compression has been integrated into the PitlinkPQC system alongside LZ4 compression. The system now supports:

- **LZ4**: Fast compression, lower compression ratio
- **Zstd**: Balanced compression, better compression ratio
- **Auto**: Automatically selects based on data characteristics and network conditions

## Features

### Compression Algorithm Selection

The system supports three modes:

1. **LZ4** - Best for:
   - Small files (< 1MB)
   - Good network conditions
   - Low latency requirements

2. **Zstd** - Best for:
   - Large files (> 1MB)
   - Poor network conditions
   - Bandwidth optimization priority

3. **Auto** - Intelligently selects:
   - Zstd for files > 1MB or network quality < 0.5
   - LZ4 for smaller files or good networks

## Usage

### Basic Usage (Auto Selection)

```rust
use trackshift::integration::*;

let pipeline = IntegratedTelemetryPipeline::new(
    "models/slm.onnx",
    "models/embedder.onnx",
    true,  // encryption
    true,  // compression
)?;

let processed = pipeline.process_chunk_full(chunk_data, network_metrics)?;
```

### Explicit Zstd Selection

```rust
use trackshift::integration::*;

let pipeline = IntegratedTelemetryPipeline::with_compression_algorithm(
    "models/slm.onnx",
    "models/embedder.onnx",
    true,  // encryption
    true,  // compression
    CompressionAlgorithm::Zstd,  // Use Zstd
)?;

let processed = pipeline.process_chunk_full(chunk_data, network_metrics)?;

// Check which algorithm was used
if let Some(algo) = processed.compression_algorithm {
    println!("Used compression: {:?}", algo);
}
```

### Manual Compression/Decompression

```rust
use trackshift::{compress_data, decompress_data, CompressionAlgorithm};

// Compress data
let compressed = compress_data(data, CompressionAlgorithm::Zstd)?;

// Decompress data
let decompressed = decompress_data(&compressed, CompressionAlgorithm::Zstd)?;
```

## Compression Settings

### Zstd Compression Level

Currently using level 3, which provides a good balance between:
- **Speed**: Fast compression/decompression
- **Ratio**: Good compression ratio

To change the level, modify `brain/src/integration.rs`:

```rust
CompressionAlgorithm::Zstd => {
    use zstd::encode_all;
    encode_all(data, 5) // Higher level = better ratio, slower
        .context("Zstd compression failed")
}
```

### LZ4 Format

Using `compress_prepend_size` format which:
- Prepends the original size to compressed data
- Allows decompression without knowing original size
- Compatible with `decompress_size_prepended`

## Performance Characteristics

| Algorithm | Compression Speed | Decompression Speed | Ratio | Best For |
|-----------|------------------|---------------------|-------|----------|
| LZ4       | Very Fast        | Very Fast           | Lower | Small files, good networks |
| Zstd (L3) | Fast             | Very Fast           | Better | Large files, poor networks |

## Integration with AI Decisions

The compression algorithm selection integrates with AI decisions:

1. **AI recommends compression** → System applies compression
2. **Auto mode** → AI considers:
   - Data size
   - Network quality score
   - Throughput requirements
3. **Result** → Optimal compression algorithm selected

## Example: Compression Comparison

```rust
let test_data = vec![0u8; 1_000_000]; // 1MB of zeros (highly compressible)

// LZ4 compression
let lz4_compressed = compress_data(&test_data, CompressionAlgorithm::Lz4)?;
println!("LZ4: {} bytes ({}% reduction)", 
    lz4_compressed.len(),
    100 - (lz4_compressed.len() * 100 / test_data.len()));

// Zstd compression
let zstd_compressed = compress_data(&test_data, CompressionAlgorithm::Zstd)?;
println!("Zstd: {} bytes ({}% reduction)",
    zstd_compressed.len(),
    100 - (zstd_compressed.len() * 100 / test_data.len()));
```

## Benefits

1. **Bandwidth Savings**: Zstd provides better compression ratios
2. **Network Adaptation**: Auto mode adapts to network conditions
3. **Flexibility**: Choose algorithm based on requirements
4. **Transparency**: Compression algorithm tracked in `ProcessedChunk`

## Dependencies

- `zstd = "0.13"` - Zstd compression library
- `lz4_flex = "0.11"` - LZ4 compression library

Both are already included in `brain/Cargo.toml`.

## Testing

Run compression tests:

```bash
cargo test -p trackshift --lib integration::tests
```

## Future Enhancements

Potential improvements:
- Configurable Zstd compression levels per use case
- Compression ratio prediction before compression
- Adaptive level selection based on data characteristics
- Streaming compression for very large files

