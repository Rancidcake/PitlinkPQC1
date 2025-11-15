//! Dashboard server main entry point

use dashboard::DashboardServer;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Starting PitlinkPQC Dashboard...");
    
    let port = std::env::var("DASHBOARD_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    
    let server = DashboardServer::new(port);
    server.start().await?;
    
    Ok(())
}

