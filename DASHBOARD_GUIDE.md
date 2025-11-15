# Dashboard Guide

A comprehensive web-based dashboard for monitoring the PitlinkPQC system in real-time.

## Features

### 📊 Real-time Monitoring
- Live updates every second
- Connection status indicator
- Last update timestamp
- System uptime tracking

### 📡 Network Metrics
- Current network path (WiFi/5G/Starlink/Multipath)
- RTT (Round Trip Time)
- Jitter
- Packet loss rate
- Throughput (Mbps)
- WiFi signal strength
- Network quality score (color-coded)

### 🧠 AI Decisions
- Route decision
- Severity classification
- Should send flag
- Similarity score (redundancy detection)
- Optimization hint
- Congestion prediction
- WFQ (Weighted Fair Queue) weights

### 🔐 QUIC-FEC Status
- Connection status
- FEC enabled/disabled
- Packets sent/received/recovered
- Handover count
- FEC configuration (data/parity shards)

### 🗜️ Compression Statistics
- Compression ratio
- LZ4 vs Zstd usage
- Total compressed/uncompressed bytes

### ⚡ Performance Metrics
- Chunks processed
- Average processing time
- AI inference time
- Total bytes sent/received

### 📈 Charts
- Network quality over time
- RTT over time
- Last 20 data points displayed

## Usage

### Start the Dashboard

```bash
# Default port 8080
cargo run --bin dashboard

# Custom port
DASHBOARD_PORT=3000 cargo run --bin dashboard
```

Then open **http://localhost:8080** in your browser.

### API Endpoints

- `GET /api/metrics/current` - Get current system metrics (JSON)
- `GET /api/metrics/history?limit=100` - Get historical metrics
- `GET /api/health` - Health check

## Integration

### Basic Integration

```rust
use dashboard::{DashboardServer, update_dashboard_metrics};
use dashboard::integration::{CompressionStats, PerformanceStats};

// Start dashboard
let server = DashboardServer::new(8080);
let collector = server.collector();

// In your main loop, update metrics
update_dashboard_metrics(
    collector.clone(),
    &network_metrics,
    &ai_decision,
    quic_connected,
    fec_enabled,
    fec_data_shards,
    fec_parity_shards,
    packets_sent,
    packets_received,
    packets_recovered,
    handover_count,
    Some(compression_stats),
    Some(performance_stats),
);
```

### With Unified Transport

```rust
use dashboard::DashboardServer;
use trackshift::UnifiedTransport;

// Start dashboard in background
let dashboard = DashboardServer::new(8080);
let collector = dashboard.collector();
tokio::spawn(async move {
    dashboard.start().await.unwrap();
});

// Use transport and update dashboard
let transport = UnifiedTransport::new(...).await?;
// ... process chunks ...

// Update dashboard with metrics
// (integration code would go here)
```

## Dashboard UI

The dashboard features a modern, responsive design with:

- **Gradient background** - Purple gradient theme
- **Card-based layout** - Organized metric cards
- **Color-coded values** - Green (good), Orange (warning), Red (bad)
- **Network path badges** - Color-coded path indicators
- **Real-time charts** - Chart.js for visualizations
- **Responsive design** - Works on desktop and mobile

## Customization

### Modify Metrics

Edit `dashboard/src/metrics.rs` to add/remove metrics.

### Customize UI

Edit `dashboard/static/index.html` to:
- Change colors and styling
- Add/remove metric cards
- Modify chart configurations
- Adjust update frequency

### Add New Charts

Add new Chart.js instances in the HTML:

```javascript
const newChart = new Chart(ctx, {
    type: 'line',
    data: { ... },
    options: { ... }
});
```

## Architecture

```
Dashboard Server (Axum)
    ├── API Routes (/api/*)
    │   ├── /metrics/current
    │   ├── /metrics/history
    │   └── /health
    │
    ├── Metrics Collector
    │   ├── Current metrics
    │   └── Historical data (last 1000)
    │
    └── Web UI
        ├── HTML/CSS/JS
        └── Chart.js visualizations
```

## Metrics Structure

All metrics are stored in `SystemMetrics`:

```rust
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub network: NetworkMetrics,
    pub ai_decision: AiDecisionMetrics,
    pub quic_fec: QuicFecMetrics,
    pub compression: CompressionMetrics,
    pub performance: PerformanceMetrics,
}
```

## Performance

- **Update frequency**: 1 second (configurable)
- **History size**: 1000 metrics (configurable)
- **Memory usage**: ~10-50 MB (depends on history size)
- **CPU usage**: Minimal (< 1% on modern hardware)

## Troubleshooting

### Dashboard not loading
- Check if port 8080 is available
- Verify firewall settings
- Check console for errors

### No metrics showing
- Ensure metrics are being updated via `collector.update()`
- Check browser console for API errors
- Verify CORS settings if accessing from different origin

### Charts not updating
- Check browser console for JavaScript errors
- Verify Chart.js is loading
- Check network tab for API responses

## License

[Your License Here]

