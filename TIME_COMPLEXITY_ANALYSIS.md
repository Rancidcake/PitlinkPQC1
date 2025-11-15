# Time Complexity Analysis - PitlinkPQC System

Comprehensive time complexity analysis of all major operations in the PitlinkPQC system.

## Overview

This document analyzes the time complexity (Big O notation) of key algorithms and operations across all components.

## 1. Telemetry AI System

### 1.1 Embedding Generation (`embed_chunk`)

**Operation**: Convert telemetry chunk to 128-dimensional embedding

**Algorithm**:
- Convert bytes to floats: O(n) where n = min(chunk_size, 1024)
- Pad to 1024: O(1) if already 1024, O(k) where k = 1024 - n
- ONNX inference: O(M) where M = model complexity

**Time Complexity**: **O(n + M)**
- Best case: O(1024 + M) ≈ O(M) for typical chunks
- Worst case: O(1024 + M) (same, since we cap at 1024)
- Space: O(1) - fixed size arrays

**Note**: M depends on embedder model architecture (typically O(10^5 - 10^6) operations)

### 1.2 Vector Store Operations

#### Insert (`insert`)
**Operation**: Add embedding to vector store

**Algorithm**:
- Validate dimension: O(1)
- Copy embedding: O(128) = O(1)
- HashMap insert: O(1) average, O(n) worst case

**Time Complexity**: **O(1)** average, **O(n)** worst case
- Space: O(1) per embedding

#### Query (`query`) - Top-K Nearest Neighbors

**Operation**: Find k most similar embeddings

**Algorithm**:
1. Compute cosine similarity for all embeddings: O(n × 128) = O(n)
2. Sort similarities: O(n log n)
3. Select top-k: O(k)

**Time Complexity**: **O(n log n)** where n = number of stored embeddings
- Space: O(n) for similarity array, O(k) for results

**Current Implementation**: Simple linear search
- **Optimization Potential**: Use HNSW (Hierarchical Navigable Small World) for O(log n) query time

#### Cosine Similarity (`cosine_similarity`)

**Operation**: Compute similarity between two 128-dim vectors

**Algorithm**:
- Dot product: O(128) = O(1)
- Norms: O(128) each = O(1)
- Division: O(1)

**Time Complexity**: **O(1)** (constant time, 128 is fixed)
- Space: O(1)

### 1.3 AI Decision (`ai_decide`)

**Operation**: Make routing/scheduling decision from 270 features

**Algorithm**:
1. Preprocess features: O(270) = O(1)
2. ONNX inference: O(M) where M = model complexity
3. Parse outputs: O(7) = O(1)

**Time Complexity**: **O(M)**
- M depends on SLM model (typically O(10^5 - 10^6) operations)
- Space: O(1) - fixed size arrays

### 1.4 Complete Chunk Processing (`process_chunk`)

**Operation**: Full pipeline from chunk to decision

**Algorithm**:
1. Network quality assessment: O(1)
2. Embed chunk: O(n + M_embed)
3. Check redundancy: O(m log m) where m = stored embeddings
4. Get context: O(m log m)
5. AI decision: O(M_slm)
6. Post-processing: O(1)

**Time Complexity**: **O(n + m log m + M_embed + M_slm)**
- n = chunk size (capped at 1024)
- m = number of stored embeddings
- M_embed = embedder model complexity
- M_slm = decision model complexity

**Typical values**:
- n ≤ 1024 (constant)
- m grows over time (bounded by memory)
- M_embed ≈ 10^5 operations
- M_slm ≈ 10^6 operations

**Practical**: **O(m log m)** dominates for large m, but m is typically < 10,000

## 2. Forward Error Correction (FEC)

### 2.1 FEC Encoding (`encode`)

**Operation**: Encode data into shards with parity

**Algorithm**:
1. Calculate shard size: O(1)
2. Pad data: O(n) where n = data size
3. Create shards: O(total_shards × shard_size)
4. Reed-Solomon encode: O(data_shards × parity_shards × shard_size)

**Time Complexity**: **O(n + k × p × s)**
- n = data size
- k = data_shards
- p = parity_shards
- s = shard_size

**For telemetry config** (8 data, 3 parity):
- **O(n + 8 × 3 × s)** = **O(n + 24s)** ≈ **O(n)** for typical data

**Space Complexity**: O(n + k×s + p×s) = O(n)

### 2.2 FEC Decoding (`decode`)

**Operation**: Reconstruct data from received shards

**Algorithm**:
1. Check if enough shards: O(1)
2. Reed-Solomon reconstruct: O(k × p × s)
3. Concatenate shards: O(k × s)

**Time Complexity**: **O(k × p × s)**
- k = data_shards
- p = parity_shards
- s = shard_size

**For telemetry config**: **O(8 × 3 × s)** = **O(24s)** ≈ **O(s)**

**Space Complexity**: O(k×s + p×s) = O((k+p)×s)

### 2.3 Add Shard (`add_shard`)

**Operation**: Add received shard to decoder

**Time Complexity**: **O(1)** - HashMap insert
**Space Complexity**: O(1) per shard

## 3. Compression

### 3.1 LZ4 Compression

**Operation**: Compress data using LZ4

**Algorithm**: LZ4 uses hash-based matching

**Time Complexity**: **O(n)** where n = input size
- Best case: O(n) - no matches
- Worst case: O(n) - linear scan
- Average: O(n)

**Space Complexity**: O(n) - output size ≤ input size

### 3.2 Zstd Compression

**Operation**: Compress data using Zstd

**Algorithm**: Zstd uses multiple compression techniques

**Time Complexity**: **O(n)** where n = input size
- Best case: O(n)
- Worst case: O(n log n) for some modes
- Average: O(n)

**Space Complexity**: O(n) - output typically 30-50% of input

### 3.3 Decompression

**Time Complexity**: **O(n)** for both LZ4 and Zstd
**Space Complexity**: O(n) - original size

## 4. Blake3 Hashing

### 4.1 Hash Computation (`blake3_hash`)

**Operation**: Compute Blake3 hash of data

**Algorithm**: Blake3 uses tree hashing

**Time Complexity**: **O(n)** where n = data size
- Parallelizable: O(n/p) with p processors
- Very fast in practice (faster than SHA-256)

**Space Complexity**: O(1) - constant space

### 4.2 Keyed Hash (`blake3_keyed_hash`)

**Time Complexity**: **O(n)** - same as regular hash
**Space Complexity**: O(1)

### 4.3 Key Derivation (`blake3_derive_key`)

**Time Complexity**: **O(n)** where n = input key material size
**Space Complexity**: O(1) - fixed output size (32 bytes)

## 5. QUIC-FEC Packet Operations

### 5.1 Packet Serialization (`to_bytes`)

**Operation**: Convert packet to bytes

**Time Complexity**: **O(1)** - fixed header size + O(data_size)
- Header: O(16) = O(1)
- Data: O(d) where d = data size

**Total**: **O(d)** where d = packet data size

### 5.2 Packet Deserialization (`from_bytes`)

**Time Complexity**: **O(d)** where d = packet size
- Parse header: O(1)
- Verify checksum: O(d) - Blake3 hash
- Copy data: O(d)

### 5.3 Packet Verification (`verify`)

**Time Complexity**: **O(d)** - Blake3 hash computation
**Space Complexity**: O(1)

## 6. Network Handover

### 6.1 Path Metrics Update (`update_path_metrics`)

**Operation**: Update metrics for a network path

**Time Complexity**: **O(1)** - HashMap insert/update
**Space Complexity**: O(1) per path

### 6.2 Handover Decision (`should_handover`)

**Operation**: Determine if handover is needed

**Algorithm**:
1. Get current path metrics: O(1)
2. Find best path: O(p) where p = number of paths
3. Compare scores: O(1)
4. Check time constraints: O(1)

**Time Complexity**: **O(p)** where p = number of available paths (typically 2-4)
- **O(1)** in practice (constant number of paths)

### 6.3 Path Score Calculation (`score`)

**Operation**: Calculate quality score for a path

**Time Complexity**: **O(1)** - fixed number of operations
**Space Complexity**: O(1)

## 7. Dashboard

### 7.1 Metrics Update (`update`)

**Operation**: Update current metrics

**Time Complexity**: **O(1)** - HashMap insert
**Space Complexity**: O(1) per update

### 7.2 Get Current Metrics (`get_current`)

**Time Complexity**: **O(1)** - HashMap read
**Space Complexity**: O(1)

### 7.3 Get History (`get_history`)

**Operation**: Retrieve historical metrics

**Time Complexity**: **O(k)** where k = limit or history size
- VecDeque iteration: O(k)
- Cloning: O(k × m) where m = metrics struct size

**Total**: **O(k × m)** ≈ **O(k)** (m is constant)

**Space Complexity**: O(k × m)

### 7.4 API Endpoint (`get_current_metrics`)

**Time Complexity**: **O(1)** - single metrics retrieval
**Space Complexity**: O(1) - JSON serialization overhead

## 8. Unified Transport Pipeline

### 8.1 Complete Processing (`process_and_send`)

**Operation**: Full pipeline from chunk to network

**Algorithm**:
1. AI processing: O(n + m log m + M)
2. Compression (if needed): O(n)
3. QUIC-FEC encoding: O(n)
4. Network send: O(n) - depends on network

**Time Complexity**: **O(n + m log m + M)**
- n = chunk size
- m = stored embeddings
- M = AI model complexity

**Network send**: O(n) but depends on bandwidth (not CPU)

**Space Complexity**: O(n) - compressed + encoded data

## 9. Overall System Complexity

### 9.1 Single Chunk Processing

**End-to-End Time Complexity**: **O(n + m log m + M)**

Where:
- **n** = chunk size (typically ≤ 1024 bytes) → **O(1)** in practice
- **m** = number of stored embeddings (grows over time)
- **M** = AI model inference (constant for fixed models)

**Dominant Terms**:
- For small m (< 1000): **O(M)** - AI inference dominates
- For large m (> 10000): **O(m log m)** - vector search dominates

**Practical Performance**:
- Small system (m < 1000): ~1-5ms per chunk
- Medium system (m < 10000): ~5-20ms per chunk
- Large system (m > 10000): ~20-100ms per chunk

### 9.2 Memory Complexity

**Per Chunk**: O(n) where n = chunk size
**Persistent**: O(m × 128 × 4) bytes for embeddings (m × 512 bytes)

**Total Memory**: O(m) where m = number of stored embeddings

### 9.3 Scalability Analysis

**Bottlenecks**:
1. **Vector Store Query**: O(m log m) - can be optimized to O(log m) with HNSW
2. **AI Inference**: O(M) - fixed, cannot be optimized without model changes
3. **FEC Encoding**: O(n) - linear, acceptable

**Optimization Opportunities**:
1. Replace simple vector store with HNSW: O(m log m) → O(log m)
2. Batch processing: Amortize overhead
3. Parallel processing: Independent chunks can be processed in parallel

## 10. Summary Table

| Operation | Time Complexity | Space Complexity | Notes |
|-----------|----------------|------------------|-------|
| Embed chunk | O(n + M_embed) | O(1) | n ≤ 1024 |
| Vector insert | O(1) avg | O(1) | HashMap |
| Vector query | O(m log m) | O(m) | Can optimize to O(log m) |
| AI decision | O(M_slm) | O(1) | Model-dependent |
| FEC encode | O(n) | O(n) | Linear in data size |
| FEC decode | O(k×p×s) | O((k+p)×s) | k=data, p=parity shards |
| LZ4 compress | O(n) | O(n) | Linear |
| Zstd compress | O(n) | O(n) | Linear |
| Blake3 hash | O(n) | O(1) | Very fast |
| Handover decision | O(p) | O(1) | p = paths (constant) |
| Packet serialize | O(d) | O(d) | d = data size |
| Complete pipeline | O(n + m log m + M) | O(n + m) | Dominated by AI + vector search |

**Key**: 
- n = data/chunk size
- m = number of stored embeddings
- M = AI model complexity
- k = data shards
- p = parity shards
- s = shard size
- d = packet data size

## 11. Performance Recommendations

1. **Use HNSW for vector store**: Reduces query from O(m log m) to O(log m)
2. **Limit embedding history**: Cap m to prevent unbounded growth
3. **Batch processing**: Process multiple chunks together when possible
4. **Parallel processing**: Independent chunks can run in parallel
5. **Cache frequent queries**: Cache AI decisions for similar inputs

## 12. Real-World Performance Estimates

**Typical Chunk Processing** (m = 1000, n = 512 bytes):
- Embedding: ~1ms
- Vector query: ~5ms
- AI decision: ~2ms
- FEC encoding: ~0.1ms
- **Total**: ~8-10ms per chunk

**Throughput**: ~100-125 chunks/second (single-threaded)

**With Parallelization** (4 threads): ~400-500 chunks/second

