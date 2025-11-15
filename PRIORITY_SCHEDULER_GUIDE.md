# Priority Tagger & Scheduler Guide

## Overview

The system now includes **automatic priority tagging** and **priority-based scheduling** for telemetry data:

1. **Priority Tagger**: Automatically analyzes chunk content and assigns priority tags
2. **Priority Scheduler**: Manages priority queues and schedules transmission based on WFQ weights

## Priority Tagger

### Features

- **Automatic Content Analysis**: Analyzes chunk bytes for keywords and patterns
- **Severity Integration**: Uses AI-determined severity to influence priority
- **Pattern Recognition**: Detects critical alerts, warnings, bulk data, etc.
- **Embedding-Based**: Can also use embeddings for semantic priority detection

### Priority Levels

```rust
pub enum ChunkPriority {
    Critical = 0,   // Emergency, immediate action required
    High = 64,      // Important, time-sensitive
    Normal = 128,   // Standard priority
    Low = 192,      // Background, can be delayed
    Bulk = 255,     // Bulk transfers, lowest priority
}
```

### Usage

```rust
use trackshift::{PriorityTagger, ChunkPriority, Severity};

// Create tagger
let tagger = PriorityTagger::new();

// Tag priority from chunk content
let chunk = b"Alert: Fire detected in building A";
let priority = tagger.tag_priority(chunk, None);
// Returns: ChunkPriority::Critical

// With severity override
let priority = tagger.tag_priority(chunk, Some(Severity::High));
// Returns: ChunkPriority::Critical (severity overrides)

// Tag from embedding
let embedding = [0.5f32; 128];
let priority = tagger.tag_priority_from_embedding(&embedding, None);
```

### Keyword Detection

**Critical Keywords** (→ Critical Priority):
- Alert, Critical, Emergency, Fire, Failure, Down, Error, Fatal, Crash, Outage

**High Keywords** (→ High Priority):
- Warning, Degraded, Slow, High, Exceeded, Threshold, Anomaly

**Low Keywords** (→ Low Priority):
- Status, OK, Normal, Idle, Info, Log, Debug

### Custom Keywords

```rust
let tagger = PriorityTagger::with_keywords(
    vec![b"CustomAlert", b"CustomCritical"],  // Critical keywords
    vec![b"CustomWarning"],                    // High keywords
    vec![b"CustomInfo"],                       // Low keywords
);
```

## Priority Scheduler

### Features

- **5 Priority Queues**: Critical, High, Normal, Low, Bulk
- **WFQ Integration**: Uses AI-determined WFQ weights for bandwidth allocation
- **Priority-Based Retrieval**: Always serves highest priority first
- **Statistics Tracking**: Monitors queue sizes and transmission stats

### Usage

```rust
use trackshift::{PriorityScheduler, ChunkPriority, AiDecision};

// Create scheduler
let scheduler = PriorityScheduler::new();

// Schedule chunks with priority
scheduler.schedule(
    chunk_data,
    ChunkPriority::Critical,
    Some(route_decision as u32),
)?;

// Update weights from AI decision
let decision = ai_system.process_chunk(chunk_data, metrics)?;
scheduler.update_weights(&decision);

// Get next chunk to send (priority-ordered)
while let Some(chunk) = scheduler.get_next() {
    send_chunk(&chunk.data)?;
}

// Get statistics
let stats = scheduler.stats();
println!("Critical queue: {}", stats.critical_queue_size);
println!("P0 weight: {}%", stats.p0_weight);
```

### Queue Management

The scheduler maintains 5 separate queues:

1. **Critical Queue** (Priority 0-31)
   - Emergency alerts
   - System failures
   - Critical errors

2. **High Queue** (Priority 32-95)
   - Warnings
   - Important metrics
   - Time-sensitive data

3. **Normal Queue** (Priority 96-159)
   - Regular telemetry
   - Standard metrics
   - Default priority

4. **Low Queue** (Priority 160-223)
   - Status updates
   - Background data
   - Non-urgent information

5. **Bulk Queue** (Priority 224-255)
   - Large datasets
   - Batch transfers
   - Can be disabled via `p2_enable`

### WFQ Weight Management

The scheduler uses WFQ (Weighted Fair Queue) weights from AI decisions:

```rust
AiDecision {
    wfq_p0_weight: 50,  // Critical/High priority bandwidth %
    wfq_p1_weight: 30,  // Normal priority bandwidth %
    wfq_p2_weight: 20,  // Low/Bulk priority bandwidth %
    p2_enable: true,     // Allow bulk transfers?
}
```

**Weight Allocation**:
- **P0** (Critical + High queues): Gets `wfq_p0_weight%` of bandwidth
- **P1** (Normal queue): Gets `wfq_p1_weight%` of bandwidth
- **P2** (Low + Bulk queues): Gets `wfq_p2_weight%` of bandwidth (if enabled)

### Retrieval Algorithm

1. **Always serve Critical first** (highest priority)
2. **Then High priority**
3. **Apply WFQ weights** for Normal/Low/Bulk queues
4. **Respect p2_enable flag** (skip bulk if disabled)

## Complete Integration Example

```rust
use trackshift::*;
use std::sync::Arc;

// Initialize components
let ai_system = TelemetryAi::new("models/slm.onnx", "models/embedder.onnx")?;
let priority_tagger = Arc::new(PriorityTagger::new());
let scheduler = Arc::new(PriorityScheduler::new());

// Process telemetry loop
loop {
    // 1. Receive telemetry chunk
    let chunk_data = receive_telemetry()?;
    
    // 2. Get AI decision
    let decision = ai_system.process_chunk(&chunk_data, network_metrics.clone())?;
    
    // 3. Tag priority automatically
    let priority = priority_tagger.tag_priority(&chunk_data, Some(decision.severity));
    
    // 4. Update scheduler weights
    scheduler.update_weights(&decision);
    
    // 5. Schedule chunk
    if decision.should_send {
        scheduler.schedule(
            chunk_data,
            priority,
            Some(decision.route as u32),
        )?;
    }
    
    // 6. Send scheduled chunks (priority-ordered)
    while let Some(scheduled) = scheduler.get_next() {
        send_data(&scheduled.data, scheduled.route)?;
    }
}
```

## Statistics & Monitoring

```rust
let stats = scheduler.stats();

println!("Queue Status:");
println!("  Critical: {}", stats.critical_queue_size);
println!("  High: {}", stats.high_queue_size);
println!("  Normal: {}", stats.normal_queue_size);
println!("  Low: {}", stats.low_queue_size);
println!("  Bulk: {}", stats.bulk_queue_size);

println!("WFQ Configuration:");
println!("  P0 Weight: {}%", stats.p0_weight);
println!("  P1 Weight: {}%", stats.p1_weight);
println!("  P2 Weight: {}%", stats.p2_weight);
println!("  P2 Enabled: {}", stats.p2_enabled);

println!("Statistics:");
println!("  Total Scheduled: {}", stats.total_scheduled);
println!("  Total Sent: {}", stats.total_sent);
```

## Benefits

### 1. Automatic Priority Assignment
- ✅ No manual priority tagging needed
- ✅ Content-aware priority detection
- ✅ Severity-based override

### 2. Intelligent Scheduling
- ✅ Priority-ordered transmission
- ✅ WFQ bandwidth allocation
- ✅ Network-aware (respects p2_enable)

### 3. Better Resource Management
- ✅ Critical data always sent first
- ✅ Bulk transfers can be throttled
- ✅ Queue statistics for monitoring

## Priority Tagging Examples

### Example 1: Critical Alert
```rust
let chunk = b"Alert: Fire detected in building A, Floor 3";
let priority = tagger.tag_priority(chunk, None);
// Result: ChunkPriority::Critical (0)
```

### Example 2: Warning
```rust
let chunk = b"Warning: High CPU usage detected - 95%";
let priority = tagger.tag_priority(chunk, None);
// Result: ChunkPriority::High (64)
```

### Example 3: Normal Telemetry
```rust
let chunk = b"Temperature: 25.5C, Humidity: 60%";
let priority = tagger.tag_priority(chunk, None);
// Result: ChunkPriority::Normal (128)
```

### Example 4: Bulk Data
```rust
let chunk = b"[1000 data points: 1,2,3,4,5,6,7,8,9,10...]";
let priority = tagger.tag_priority(chunk, None);
// Result: ChunkPriority::Bulk (255)
```

## Scheduler Behavior Examples

### Scenario 1: Critical Alert During Normal Operation
```
Queue State:
  Critical: 1 chunk
  Normal: 10 chunks
  
Retrieval Order:
  1. Critical chunk (served immediately)
  2. Normal chunks (served in order)
```

### Scenario 2: Network Congestion (P2 Disabled)
```
AI Decision: p2_enable = false
Queue State:
  Critical: 2 chunks
  High: 5 chunks
  Normal: 20 chunks
  Bulk: 100 chunks
  
Retrieval Order:
  1-2. Critical chunks
  3-7. High priority chunks
  8-27. Normal chunks
  (Bulk queue ignored - p2_enable = false)
```

### Scenario 3: WFQ Weight Distribution
```
AI Decision:
  p0_weight: 70% (Critical/High)
  p1_weight: 20% (Normal)
  p2_weight: 10% (Low/Bulk)
  
Retrieval Pattern:
  - 7 chunks from Critical/High queues
  - 2 chunks from Normal queue
  - 1 chunk from Low/Bulk queues
  (Repeats)
```

## Testing

Run the example:

```bash
cargo run --example priority_scheduler -p trackshift
```

Run tests:

```bash
cargo test -p trackshift priority_tagger
cargo test -p trackshift scheduler
```

## Summary

✅ **Priority Tagger**: Automatically tags chunks based on content analysis
✅ **Priority Scheduler**: Manages 5 priority queues with WFQ support
✅ **AI Integration**: Uses AI decisions for weight management
✅ **Statistics**: Comprehensive monitoring and stats
✅ **Production Ready**: Thread-safe, efficient, well-tested

The system now provides **automatic priority management** from tagging to scheduling!

