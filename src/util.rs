use crate::GlueDataCatalog;
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_util::MetricKindMask;
use rmcp::transport::sse_server::SseServer;
use std::net::SocketAddr;
use std::time::Duration;
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

/// Sets up logging with tracing
pub fn setup_logging() {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
}

pub fn setup_metrics() {
    let builder = PrometheusBuilder::new();
    builder
        .idle_timeout(
            MetricKindMask::COUNTER | MetricKindMask::HISTOGRAM,
            Some(Duration::from_secs(60)),
        )
        .install()
        .expect("failed to install Prometheus recorder");
}

/// Starts the SSE server with the GlueDataCatalog service
pub async fn start_server(
    bind_address: &str,
) -> anyhow::Result<tokio_util::sync::CancellationToken> {
    // Log server startup
    info!("Starting server on {}", bind_address);

    let service = GlueDataCatalog::from_env().await;
    let addr: SocketAddr = bind_address.parse()?;

    let ct = SseServer::serve(addr)
        .await?
        .with_service(move || service.clone());

    Ok(ct)
}
