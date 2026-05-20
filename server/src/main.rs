use rustmc_server::network;
use rustmc_server::server_config::ServerConfig;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("rustmc_server=info".parse()?))
        .init();

    tracing::info!("Starting RustMC Server v{}", env!("CARGO_PKG_VERSION"));

    let mut config = ServerConfig::load();

    let addr = std::env::var("RUSTMC_BIND").unwrap_or_else(|_| config.server.bind.clone());
    if let Some(vd) = std::env::var("RUSTMC_VIEW_DISTANCE")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        config.server.view_distance = vd;
    }

    let mut bridge = plugin_bridge::PluginBridge::new();
    let plugins_dir =
        std::env::var("RUSTMC_PLUGINS").unwrap_or_else(|_| config.server.plugins_directory.clone());
    match bridge.load_plugins(&plugins_dir) {
        Ok(count) => tracing::info!("Loaded {count} plugin(s)"),
        Err(e) => tracing::warn!("Plugin loading failed: {e}"),
    }

    let server = network::Server::new(addr, config);
    server.run().await
}
