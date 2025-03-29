//! AureaCore service catalog

use tracing::info;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("AureaCore service catalog starting up...");
} 