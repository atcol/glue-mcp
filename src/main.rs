mod util;

use glue_mcp::GlueDataCatalog;
use tracing::info;

const BIND_ADDRESS: &str = "127.0.0.1:8000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    util::setup_logging();
    util::setup_metrics();

    info!("Metrics & logging initialised");

    let ct = util::start_server(BIND_ADDRESS).await?;

    tokio::signal::ctrl_c().await?;
    info!("Shutdown signal received, stopping server");
    ct.cancel();
    info!("Server stopped");
    Ok(())
}
