# PitlinkPQC Dashboard

A web-based dashboard for monitoring and visualizing the PitlinkPQC system in real-time.

## Features

- 📊 **Real-time Metrics**: Live updates of system metrics
- 📡 **Network Monitoring**: Network quality, RTT, jitter, throughput
- 🧠 **AI Decisions**: View AI routing and optimization decisions
- 🔐 **QUIC-FEC Status**: Connection status, FEC metrics, handover events
- 🗜️ **Compression Stats**: Compression ratios and algorithm usage
- ⚡ **Performance Metrics**: Processing times, throughput
- 📈 **Charts**: Visualize network quality and RTT over time

## Usage

### Start the Dashboard

```bash
# Default port 8080
cargo run --bin dashboard

# Custom port
DASHBOARD_PORT=3000 cargo run --bin dashboard
```

Then open http://localhost:8080 in your browser.

### API Endpoints

- `GET /api/metrics/current` - Get current system metrics
- `GET /api/metrics/history?limit=100` - Get historical metrics
- `GET /api/health` - Health check

## Integration

The dashboard uses a `MetricsCollector` to gather metrics from the system:

```rust
use dashboard::{DashboardServer, MetricsCollector};
use dashboard::metrics::SystemMetrics;

// Create dashboard server
let server = DashboardServer::new(8080);
let collector = server.collector();

// Update metrics from your system
let metrics = SystemMetrics {
    // ... populate with actual metrics
    ..Default::default()
};
collector.update(metrics);
```

## Metrics Collected

### Network Metrics
- RTT, jitter, packet loss
- Throughput
- Signal strength (WiFi/5G)
- Current network path
- Network quality score

### AI Decision Metrics
- Route decision
- Severity classification
- Should send flag
- Similarity score
- Optimization hint
- Congestion prediction
- WFQ weights

### QUIC-FEC Metrics
- Connection status
- FEC configuration
- Packet statistics
- Handover count
- Recovery statistics

### Compression Metrics
- Compression ratio
- Algorithm usage (LZ4/Zstd)
- Total compressed/uncompressed bytes

### Performance Metrics
- Chunks processed
- Processing times
- AI inference time
- Bytes sent/received
- Uptime

## Dashboard UI

The dashboard provides:
- **Status Bar**: Connection status, last update, uptime
- **Network Card**: Current network metrics
- **AI Decisions Card**: Latest AI decisions
- **QUIC-FEC Card**: Connection and FEC status
- **Compression Card**: Compression statistics
- **Performance Card**: System performance metrics
- **Charts**: Network quality and RTT over time

## Customization

The dashboard UI is in `dashboard/static/index.html`. You can customize:
- Colors and styling
- Chart types and configurations
- Metrics displayed
- Update frequency

## License

[Your License Here]

