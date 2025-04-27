use rmcp::transport::sse_server::SseServer;
use glue_mcp::GlueDataCatalog;
use tracing::{info, Level};
use tracing_subscriber::{FmtSubscriber, EnvFilter};
use env_logger;

const BIND_ADDRESS: &str = "127.0.0.1:8000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
    
    // Log server startup
    info!("Starting server on {}", BIND_ADDRESS);

    let service = GlueDataCatalog::from_env().await;
    
    let ct = SseServer::serve(BIND_ADDRESS.parse()?)
        .await?
        .with_service(move || service.clone());
    
    info!("Server started successfully, press Ctrl+C to stop");
    
    tokio::signal::ctrl_c().await?;
    info!("Shutdown signal received, stopping server");
    ct.cancel();
    info!("Server stopped");
    Ok(())
}
