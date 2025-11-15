# Priority Tagger Status

## Answer: **NO, there is NO automatic priority tagger**

The system does **NOT** have an automatic priority tagger that analyzes telemetry chunks and assigns priority tags. Priority must be **manually assigned** or provided as input.

## Current Priority Handling

### 1. **Buffering Priority** (Manual Assignment)

When buffering data during network outages, priority must be **manually provided**:

```rust
// Example from patchy_network_example.rs
let priority = if decision.severity == Severity::High { 
    0      // Highest priority
} else { 
    128    // Medium priority
};
buffer.add(chunk_data.to_vec(), priority)?;
```

**Priority Range**: `u8` (0 = highest, 255 = lowest)

### 2. **File Transfer Priority** (Input Parameter)

For file transfers, priority is provided as an **input parameter**:

```rust
let decision = ai_system.process_file_transfer(
    chunk_data,
    network_metrics,
    file_size,
    file_type,
    file_priority,  // <-- Must be provided
    bytes_transferred,
    requires_integrity_check,
    integrity_check_failures,
    is_resume,
)?;
```

**FilePriority Enum**:
- `Critical = 0` - Emergency, real-time critical
- `High = 1` - Important, time-sensitive
- `Normal = 2` - Standard priority
- `Low = 3` - Background, bulk transfers

### 3. **Queue Priority** (AI-Determined Bandwidth Allocation)

The AI model outputs **WFQ (Weighted Fair Queue) weights** for three priority queues:

```rust
AiDecision {
    wfq_p0_weight: u32,  // Priority 0 bandwidth % (0-100)
    wfq_p1_weight: u32,  // Priority 1 bandwidth % (0-100)
    wfq_p2_weight: u32,  // Priority 2 bandwidth % (0-100)
    p2_enable: bool,     // Whether to allow bulk transfers
}
```

**Note**: These are bandwidth allocation weights, NOT priority tags for individual chunks.

## What's Missing: Automatic Priority Tagger

A priority tagger would:
1. **Analyze chunk content** (using embeddings or pattern matching)
2. **Automatically assign priority** based on:
   - Content type (alerts vs. regular telemetry)
   - Keywords (e.g., "Alert", "Error", "Critical")
   - Data patterns
   - Severity indicators
3. **Tag chunks** with priority before processing

## Current Workaround

Priority is currently assigned based on:
- **Severity** (High severity → priority 0, Low → priority 128)
- **Manual assignment** by the application
- **File type/priority** (for file transfers)

## Example: How Priority is Currently Used

```rust
// 1. Process chunk with AI
let decision = ai_system.process_chunk(chunk_data, network_metrics)?;

// 2. Manually assign priority based on severity
let priority = match decision.severity {
    Severity::High => 0,    // Critical - highest priority
    Severity::Low => 128,   // Normal - medium priority
};

// 3. Buffer with assigned priority
if decision.should_buffer {
    buffer.add(chunk_data.to_vec(), priority)?;
}
```

## Recommendation: Add Priority Tagger

To add automatic priority tagging, you could:

1. **Content-Based Tagger**:
   ```rust
   fn tag_priority(chunk_data: &[u8]) -> u8 {
       // Analyze content for keywords
       if chunk_data.contains(b"Alert") || chunk_data.contains(b"Critical") {
           0  // Highest priority
       } else if chunk_data.contains(b"Warning") {
           64  // High priority
       } else {
           128  // Normal priority
       }
   }
   ```

2. **Embedding-Based Tagger**:
   ```rust
   fn tag_priority_from_embedding(embedding: &[f32; 128]) -> u8 {
       // Use ML model to classify priority from embedding
       // Could use a separate ONNX model or simple heuristics
   }
   ```

3. **Pattern-Based Tagger**:
   ```rust
   fn tag_priority_from_patterns(chunk: &[u8], patterns: &[(&[u8], u8)]) -> u8 {
       // Match against known patterns
       for (pattern, priority) in patterns {
           if chunk.windows(pattern.len()).any(|w| w == *pattern) {
               return *priority;
           }
       }
       128  // Default
   }
   ```

## Summary

| Aspect | Status | Details |
|--------|--------|---------|
| **Automatic Priority Tagger** | ❌ **NO** | Does not exist |
| **Manual Priority Assignment** | ✅ **YES** | Must be done by application |
| **Priority-Based Buffering** | ✅ **YES** | Uses manually assigned priority |
| **File Transfer Priority** | ✅ **YES** | Provided as input parameter |
| **Queue Priority Weights** | ✅ **YES** | AI-determined bandwidth allocation |

**Conclusion**: The system supports priority handling but requires **manual priority assignment**. There is no automatic priority tagger that analyzes content and assigns priority tags automatically.

