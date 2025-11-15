# Medical, Disaster & Multi-Scenario Data Support

## ✅ Comprehensive Support for All Scenarios

The system now provides **complete support** for medical data, disaster response, and all deployment scenarios with intelligent priority tagging and real-time status monitoring.

## Medical Data Support

### Supported Medical Formats

#### HL7 (Health Level 7)
- **Format Detection**: Starts with `MSH|` (Message Header)
- **Priority**: Critical for alerts, High for patient data
- **Example**: `MSH|^~\&|SendingApp|SendingFacility|ReceivingApp|ReceivingFacility|20240101120000||ADT^A01|12345|P|2.5`

#### DICOM (Digital Imaging and Communications in Medicine)
- **Format Detection**: `DICM` magic number at offset 128
- **Priority**: High (medical imaging is critical)
- **Use Cases**: X-rays, CT scans, MRIs, ultrasound

#### FHIR (Fast Healthcare Interoperability Resources)
- **Format Detection**: JSON with `resourceType` and medical keywords
- **Priority**: High for patient data, Critical for alerts
- **Resources**: Patient, Observation, DiagnosticReport, Medication

#### HL7 XML
- **Format Detection**: XML with `urn:hl7-org` namespace or `ClinicalDocument`
- **Priority**: High for patient data

### Medical Priority Logic

| Data Type | Priority | Examples |
|-----------|----------|----------|
| **Code Blue/Red** | **Critical** | Cardiac arrest, respiratory failure |
| **Emergency Surgery** | **Critical** | Life-threatening conditions |
| **Vital Signs** | **High** | Patient monitoring data |
| **Lab Results** | **High** | Diagnostic test results |
| **DICOM Images** | **High** | Medical imaging |
| **Routine Data** | **Normal** | Scheduled reports, logs |

### Medical Scenarios

#### Mobile Clinic
- **Detection**: Contains "mobile" or "field" + medical keywords
- **Priority**: Elevated for critical patient data
- **Use Case**: Field medical operations, remote clinics

#### Medical Facility
- **Detection**: Contains "hospital", "clinic", "patient", "medical"
- **Priority**: Standard medical priority logic
- **Use Case**: Hospitals, clinics, medical centers

## Disaster Response Support

### Disaster Data Formats

#### Emergency Alerts
- **Detection**: Keywords like "evacuate", "disaster", "emergency response"
- **Priority**: Critical for immediate threats, High for alerts
- **Examples**: Evacuation orders, structural collapse alerts

#### Disaster Scenarios
- **Detection**: Contains "disaster", "emergency", "evacuate", "crisis"
- **Priority**: Critical for immediate danger
- **Use Cases**: Natural disasters, emergencies, crisis response

### Disaster Priority Logic

| Threat Level | Priority | Examples |
|--------------|----------|----------|
| **Immediate Danger** | **Critical** | Evacuate, structural collapse, chemical spill |
| **Emergency Alerts** | **High** | Disaster warnings, crisis alerts |
| **Status Updates** | **Normal** | Routine status reports |

## Engineering Data Support

### Engineering Formats

#### Sensor Data
- **Detection**: Contains "sensor", "telemetry", "measurement"
- **Priority**: High for alerts, Normal for routine data
- **Use Cases**: Remote monitoring, IoT sensors

#### CAD Data
- **Detection**: Contains "cad" or CAD file formats
- **Priority**: Normal (can be large files)
- **Use Cases**: Engineering designs, blueprints

### Engineering Scenarios

#### Remote Engineering Sites
- **Detection**: Contains "engineering", "remote site", "field ops"
- **Priority**: High for critical alerts
- **Use Cases**: Construction sites, remote installations

## All Deployment Scenarios

### Scenario Detection

The system automatically detects scenarios:

| Scenario | Detection Keywords | Priority Adjustment |
|----------|-------------------|---------------------|
| **Media Studio** | "media", "broadcast", "production", "studio" | Normal (streaming-friendly) |
| **Rural Lab** | "lab", "laboratory", "research", "rural" | High for critical research |
| **Mobile Clinic** | "mobile", "field" + medical | Elevated for patient data |
| **Remote Engineering** | "engineering", "remote site", "construction" | High for alerts |
| **Disaster Site** | "disaster", "emergency", "evacuate" | Critical for threats |
| **Medical Facility** | "hospital", "clinic", "patient" | Medical priority logic |
| **Field Operations** | "field", "remote" | Context-dependent |

## Real-Time Status Monitoring

### Transfer Status Tracking

```rust
use trackshift::{RealtimeStatusMonitor, TransferStatus, IntegrityMethod, ChunkPriority};

let monitor = RealtimeStatusMonitor::new();

// Register transfer
monitor.register_transfer(
    "transfer-1".to_string(),
    1_000_000,  // 1MB
    ChunkPriority::High,
    IntegrityMethod::Blake3,
);

// Update progress
monitor.update_transfer(
    "transfer-1",
    500_000,  // 50% complete
    TransferStatus::InProgress,
    Some(10.0),  // 10 Mbps
)?;

// Get status
let status = monitor.get_transfer_status("transfer-1").unwrap();
println!("Progress: {:.1}%", status.progress * 100.0);
println!("Speed: {:.2} Mbps", status.speed_mbps);
println!("ETA: {:?} seconds", status.eta_seconds);
```

### Network Status Monitoring

```rust
use trackshift::{NetworkStatus, NetworkMetricsInput};

// Update network status
monitor.update_network_status(
    network_metrics,
    0.85,  // Quality score
    false,  // Not patchy
);

// Get network status
if let Some(network) = monitor.get_network_status() {
    println!("RTT: {:.2} ms", network.rtt_ms);
    println!("Throughput: {:.2} Mbps", network.throughput_mbps);
    println!("Quality: {:.2}", network.quality_score);
}
```

### System Health Monitoring

```rust
let health = monitor.get_system_health();
println!("Active Transfers: {}", health.active_transfers);
println!("Queue Size: {}", health.queue_size);
println!("Error Rate: {:.2}%", health.error_rate * 100.0);
```

### Comprehensive Status Snapshot

```rust
let snapshot = monitor.get_status_snapshot();

// All transfers
for transfer in &snapshot.transfers {
    println!("Transfer {}: {:.1}% - {:?}", 
        transfer.transfer_id, 
        transfer.progress * 100.0,
        transfer.status
    );
}

// Network status
if let Some(network) = &snapshot.network {
    println!("Network Quality: {:.2}", network.quality_score);
}

// Scheduler stats
if let Some(scheduler) = &snapshot.scheduler_stats {
    println!("Critical Queue: {}", scheduler.critical_queue);
    println!("High Queue: {}", scheduler.high_queue);
}
```

## Integrity Checks

### Blake3 Integrity (Recommended)

Blake3 is now the **default integrity method** for all files:
- **Fast**: Faster than SHA256
- **Secure**: Cryptographically secure
- **Efficient**: Good performance even for large files

```rust
use trackshift::IntegrityMethod;
use common::blake3_hash;

// Compute Blake3 hash
let hash = blake3_hash(data);

// Verify integrity
let received_hash = blake3_hash(received_data);
assert_eq!(hash, received_hash);
```

### Integrity Check Status

```rust
use trackshift::IntegrityCheckStatus;

// Update integrity status
monitor.update_integrity_status(
    "transfer-1",
    IntegrityCheckStatus::InProgress,
)?;

// After verification
monitor.update_integrity_status(
    "transfer-1",
    IntegrityCheckStatus::Passed,
)?;
```

## Priority Channels

### 5 Priority Levels

1. **Critical** (0) - Emergency, immediate action
2. **High** (64) - Important, time-sensitive
3. **Normal** (128) - Standard priority
4. **Low** (192) - Background, can be delayed
5. **Bulk** (255) - Bulk transfers, lowest priority

### WFQ Weight Management

Priority channels use Weighted Fair Queue (WFQ) weights:

```rust
// Update scheduler weights from AI decision
scheduler.update_weights(&ai_decision);

// Weights determine bandwidth allocation:
// P0 (Critical/High): 50% bandwidth
// P1 (Normal): 30% bandwidth
// P2 (Low/Bulk): 20% bandwidth
```

## Resilience for Unstable Links

### Forward Error Correction (FEC)

QUIC-FEC provides packet loss recovery:

```rust
use quic_fec::fec::FecConfig;

// High redundancy for unstable links
let fec_config = FecConfig::for_patchy_network();
// 4 data shards + 4 parity shards (50% redundancy)
```

### Adaptive Behaviors

The system automatically adapts to unstable links:

1. **Smaller Chunks**: Reduces chunk size on patchy networks
2. **Increased FEC**: Higher redundancy for unreliable links
3. **Buffering**: Buffers data during outages
4. **Retry Strategy**: Aggressive retries for failed transfers
5. **Priority Rebalancing**: Gives more bandwidth to critical data

### Network Quality Assessment

```rust
let network_quality = assess_network_quality(&metrics);

if network_quality.is_patchy {
    // Use smaller chunks
    // Increase FEC redundancy
    // Buffer critical data
    // Pause bulk transfers
}
```

## Complete Example: Medical Data Transfer

```rust
use trackshift::*;
use quic_fec::connection::QuicFecConnection;

// Initialize components
let ai_system = TelemetryAi::new("models/slm.onnx", "models/embedder.onnx")?;
let tagger = PriorityTagger::new();
let scheduler = PriorityScheduler::new();
let monitor = RealtimeStatusMonitor::with_scheduler(scheduler.clone());
let quic_conn = QuicFecConnection::connect(/* ... */).await?;

// Process medical data (HL7 message)
let hl7_data = b"MSH|^~\\&|SendingApp|Facility|ReceivingApp|ReceivingFacility|20240101120000||ADT^A01|12345|P|2.5";

// Get AI decision
let decision = ai_system.process_chunk(hl7_data, network_metrics.clone())?;

// Tag priority (automatically detects medical format)
let priority = tagger.tag_priority(hl7_data, Some(decision.severity));
// Result: ChunkPriority::High (patient data)

// Detect scenario
let scenario = tagger.detect_scenario(hl7_data);
// Result: DataScenario::MedicalFacility

// Register transfer
monitor.register_transfer(
    "hl7-msg-12345".to_string(),
    hl7_data.len() as u64,
    priority,
    IntegrityMethod::Blake3,
);

// Schedule with priority
scheduler.schedule(hl7_data.to_vec(), priority, Some(decision.route as u32))?;

// Send via QUIC-FEC
quic_conn.send_with_fec(
    hl7_data.to_vec().into(),
    FecConfig::for_telemetry(),
).await?;

// Update progress
monitor.update_transfer(
    "hl7-msg-12345",
    hl7_data.len() as u64,
    TransferStatus::Completed,
    Some(100.0),  // 100 Mbps
)?;

// Verify integrity
monitor.update_integrity_status(
    "hl7-msg-12345",
    IntegrityCheckStatus::Passed,
)?;
```

## Summary

| Feature | Status | Details |
|---------|--------|---------|
| **Medical Formats** | ✅ **Complete** | HL7, DICOM, FHIR, HL7 XML |
| **Disaster Response** | ✅ **Complete** | Emergency alerts, crisis data |
| **Engineering Data** | ✅ **Complete** | Sensor data, CAD files |
| **Scenario Detection** | ✅ **Complete** | 7 scenarios supported |
| **Real-Time Status** | ✅ **Complete** | Transfer, network, health monitoring |
| **Integrity Checks** | ✅ **Complete** | Blake3 (default), SHA256, CRC32 |
| **Priority Channels** | ✅ **Complete** | 5 levels + WFQ weights |
| **Unstable Links** | ✅ **Complete** | FEC, buffering, adaptive behaviors |

The system is now **production-ready** for all scenarios with comprehensive support for medical data, disaster response, and real-time monitoring!

