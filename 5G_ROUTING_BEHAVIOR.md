# 5G Routing Behavior - How It Affects the System

## Overview

When the AI system selects **5G** as the route (`RouteDecision::FiveG`), it affects system behavior in several ways. However, the **5G signal strength** (`fiveg_signal`) has more direct impact on system behavior than the route selection itself.

## How 5G Signal Affects System Behavior

### 1. Network Quality Assessment

The `fiveg_signal` value (in dBm, typically -30 to -90) is used to assess overall network quality:

```rust
// From network_quality.rs
let worst_signal = metrics.wifi_signal.min(metrics.fiveg_signal);
if worst_signal < -90.0 {
    score -= 0.3;  // Very weak signal = lower quality score
    issues += 1;
} else if worst_signal < -80.0 {
    score -= 0.15;  // Weak signal = moderate quality reduction
}
```

**Impact:**
- **Strong 5G signal** (> -70 dBm): Higher network quality score → Better performance
- **Weak 5G signal** (< -80 dBm): Lower network quality score → Triggers optimizations

### 2. Adaptive Behaviors Based on Network Quality

When network quality is assessed (which includes 5G signal strength), it triggers:

#### A. Redundancy Detection Threshold
```rust
// Adaptive threshold based on network quality
match network_quality.recommended_action {
    NetworkAction::Emergency => 0.85,   // Very aggressive (skip more)
    NetworkAction::Aggressive => 0.90,  // Aggressive (skip similar)
    NetworkAction::Normal => 0.95,      // Normal threshold
    NetworkAction::Conservative => 0.98, // Conservative (send more)
}
```

**Effect:** Poor 5G signal → Lower threshold → More aggressive redundancy detection → Saves bandwidth

#### B. Compression Decisions
```rust
// Compression recommended when network is patchy
if network_quality.is_patchy || network_quality.score < 0.7 {
    // Apply compression
}
```

**Effect:** Weak 5G signal → Patchy network detected → Compression enabled → Reduced bandwidth usage

#### C. Buffering Strategy
```rust
// Buffer data when network is too poor
if decision.should_buffer {
    // Network down or very poor - buffer for later
}
```

**Effect:** Very weak 5G signal (< -90 dBm) → Network quality < 0.3 → Data buffered instead of sent

#### D. Retry Strategy
```rust
// Retry strategy based on network quality
match network_quality.recommended_action {
    NetworkAction::Emergency => RetryStrategy::Buffer,
    NetworkAction::Aggressive => RetryStrategy::Exponential,
    NetworkAction::Normal => RetryStrategy::Immediate,
    NetworkAction::Conservative => RetryStrategy::Immediate,
}
```

**Effect:** Poor 5G signal → Exponential backoff or buffering → Prevents wasted retries

#### E. Priority Rebalancing
```rust
// Prioritize critical data only on poor networks
if network_quality.prioritize_critical_only() {
    // Only send critical priority data
}
```

**Effect:** Weak 5G signal → Only critical data sent → Better reliability for important data

### 3. File Transfer Optimizations

For file transfers, 5G signal affects:

#### A. Chunk Size Selection
```rust
decision.recommended_chunk_size = if network_quality.is_patchy {
    if file_size > 10_000_000 {
        64 * 1024  // 64KB for large files on patchy networks
    } else {
        32 * 1024  // 32KB for smaller files
    }
} else {
    1024 * 1024  // 1MB for good networks
}
```

**Effect:** Weak 5G signal → Smaller chunks → Better resume capability if transfer fails

#### B. Parallel Transfer
```rust
decision.enable_parallel_transfer = file_size > 10_000_000 && network_quality.score > 0.7;
```

**Effect:** Strong 5G signal → Parallel transfers enabled → Faster large file transfers

## Route Decision vs Signal Strength

### Route Decision (`RouteDecision::FiveG`)

The route decision is an **OUTPUT** from the AI model that indicates:
- **Which network path to use** (5G, WiFi, Starlink, or Multipath)
- This is a **recommendation** that needs to be acted upon by an external router component

**Current Implementation:**
- The route decision itself doesn't directly change internal system behavior
- It's meant to be consumed by a router component that switches network paths
- The system tracks it in `AiDecision.route` for external systems to use

### 5G Signal Strength (`fiveg_signal`)

The 5G signal strength is an **INPUT** that:
- **Directly affects** network quality assessment
- **Triggers** all the adaptive behaviors listed above
- Is used by the AI model to make routing decisions

## Example Scenarios

### Scenario 1: Strong 5G Signal (-50 dBm)

```
Input: fiveg_signal = -50.0 dBm
↓
Network Quality: score = 0.95 (excellent)
↓
Behaviors:
- Redundancy threshold: 0.98 (conservative - send more)
- Compression: Optional
- Buffering: Disabled
- Retry: Immediate
- Chunk size: 1MB (large chunks)
- Parallel transfer: Enabled (4 streams)
- Route decision: Likely FiveG
```

### Scenario 2: Weak 5G Signal (-85 dBm)

```
Input: fiveg_signal = -85.0 dBm
↓
Network Quality: score = 0.45 (poor)
↓
Behaviors:
- Redundancy threshold: 0.90 (aggressive - skip more)
- Compression: Always enabled
- Buffering: May buffer if score < 0.3
- Retry: Exponential backoff
- Chunk size: 32-64KB (small chunks)
- Parallel transfer: Disabled
- Route decision: May switch to WiFi or Multipath
```

### Scenario 3: Very Weak 5G Signal (-95 dBm)

```
Input: fiveg_signal = -95.0 dBm
↓
Network Quality: score = 0.25 (very poor)
↓
Behaviors:
- Redundancy threshold: 0.85 (very aggressive)
- Compression: Always enabled
- Buffering: Enabled (network too poor)
- Retry: Buffer and wait
- Chunk size: 32KB (smallest)
- Parallel transfer: Disabled
- Route decision: Likely WiFi or Starlink
```

## Integration with Router Component

The route decision needs to be consumed by an external router:

```rust
// Example integration
let decision = ai_system.process_chunk(chunk_data, network_metrics)?;

match decision.route {
    RouteDecision::FiveG => {
        router.switch_to_5g()?;
        // System will adapt based on fiveg_signal value
    }
    RouteDecision::WiFi => {
        router.switch_to_wifi()?;
        // System will adapt based on wifi_signal value
    }
    RouteDecision::Starlink => {
        router.switch_to_starlink()?;
        // System will adapt based on starlink_latency
    }
    RouteDecision::Multipath => {
        router.enable_multipath()?;
        // Use multiple paths simultaneously
    }
}
```

## Summary

**Does using 5G make changes?**

1. **Route Selection (`RouteDecision::FiveG`)**: 
   - This is a recommendation/output
   - Needs external router to act on it
   - Doesn't directly change internal system behavior

2. **5G Signal Strength (`fiveg_signal`)**:
   - **Directly affects** network quality assessment
   - **Triggers** adaptive behaviors:
     - Compression decisions
     - Buffering strategies
     - Retry strategies
     - Redundancy thresholds
     - Chunk sizing
     - Parallel transfer enablement
     - Priority rebalancing

**Key Takeaway:** The 5G **signal strength** has significant impact on system behavior, while the route **decision** is more of an output that guides external routing components. The system adapts its behavior based on the actual network conditions (including 5G signal strength), not just which route was selected.

