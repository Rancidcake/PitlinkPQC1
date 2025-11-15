# PitlinkPQC - Integrated Telemetry & File Transfer System

A unified Rust workspace combining AI-powered telemetry analysis, post-quantum encryption, and compression for intelligent data transfer.

## Components

- **brain (trackshift)**: AI-powered telemetry decision engine with ONNX model inference
- **dashboard**: Web-based dashboard for real-time system monitoring and visualization
- **quic_fec**: Modified QUIC protocol with Forward Error Correction (FEC) and Blake3 hashing
- **rust_pqc**: Post-quantum cryptography encryption/decryption (Kyber-768 + XChaCha20-Poly1305)
- **Compression**: LZ4 and Zstd compression support with intelligent algorithm selection
- **lz4_chunker**: LZ4 compression utilities
- **csv_lz4_tool**: CSV-specific compression tools
- **common**: Shared utilities and helpers (includes Blake3 hashing)

## Quick Start

See [INTEGRATION.md](INTEGRATION.md) for complete integration guide.

```bash
# Build all components
cargo build --release

# Start the dashboard
cargo run --bin dashboard

# Run integrated workflow example (once ORT API is fixed)
cargo run --example unified_transport -p trackshift
```

## Integration Status

See [INTEGRATION_STATUS.md](INTEGRATION_STATUS.md) for current status and known issues.

## Documentation

- **Complete System Analysis**: [COMPLETE_SYSTEM_ANALYSIS.md](COMPLETE_SYSTEM_ANALYSIS.md) - Full system review and status
- **Time Complexity Analysis**: [TIME_COMPLEXITY_ANALYSIS.md](TIME_COMPLEXITY_ANALYSIS.md) - Algorithm complexity analysis
- **Dashboard Guide**: [DASHBOARD_GUIDE.md](DASHBOARD_GUIDE.md) - Web dashboard for monitoring
- **Connection Summary**: [CONNECTION_SUMMARY.md](CONNECTION_SUMMARY.md) - How all components connect
- **QUIC-FEC Protocol**: [QUIC_FEC_README.md](QUIC_FEC_README.md) - Modified QUIC with FEC and Blake3
- **Complete Integration**: [INTEGRATION_COMPLETE.md](INTEGRATION_COMPLETE.md) - Full integration guide
- **Integration Guide**: [INTEGRATION.md](INTEGRATION.md)
- **Priority Tagger & Scheduler**: [PRIORITY_SCHEDULER_GUIDE.md](PRIORITY_SCHEDULER_GUIDE.md)
- **Data Format Support**: [DATA_FORMAT_SUPPORT.md](DATA_FORMAT_SUPPORT.md)
- **Image & Video Support**: [IMAGE_VIDEO_SUPPORT.md](IMAGE_VIDEO_SUPPORT.md)
- **Medical & Disaster Support**: [MEDICAL_DISASTER_SUPPORT.md](MEDICAL_DISASTER_SUPPORT.md)
- **Remaining Features**: [REMAINING_FEATURES.md](REMAINING_FEATURES.md)
- **Zstd Compression**: [ZSTD_COMPRESSION.md](ZSTD_COMPRESSION.md)
- **Brain Component**: [brain/README.md](brain/README.md)
- **Encryption**: [rust_pqc/README.md](rust_pqc/README.md)
- **Quick Start**: [QUICKSTART.md](QUICKSTART.md)

## Architecture

```
Telemetry Data → AI Analysis → Decision → [Compression/Encryption] → Transfer
```

The system intelligently:
- **Automatically tags priority** based on content analysis (supports medical, disaster, engineering data)
- **Schedules transmission** using priority queues and WFQ weights
- **Real-time status monitoring** for transfers, network, and system health
- **Medical data support** (HL7, DICOM, FHIR) with critical priority for alerts
- **Disaster response support** with emergency alert detection
- **Scenario detection** (media studios, rural labs, mobile clinics, remote engineering, disaster sites)
- Analyzes network quality and data patterns
- Makes routing decisions (WiFi/Starlink/Multipath)
- **Uses QUIC-FEC** for reliable transfer with Forward Error Correction
- **Seamless handover** between network paths (WiFi/Starlink)
- **Blake3 hashing** for fast integrity verification (default)
- **Resilient for unstable links** with FEC, buffering, and adaptive behaviors
- Detects redundant data (saves 30-80% bandwidth)
- Applies compression when beneficial (LZ4 or Zstd)
- Encrypts sensitive data with post-quantum security
- Handles patchy networks gracefully

## License

[Your License Here]