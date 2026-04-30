mod network;
mod protocol;
mod world;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("rustmc_server=info".parse()?))
        .init();

    tracing::info!("Starting RustMC Server v{}", env!("CARGO_PKG_VERSION"));

    let addr = std::env::var("RUSTMC_BIND").unwrap_or_else(|_| "0.0.0.0:25565".to_string());

    let mut bridge = plugin_bridge::PluginBridge::new();
    let plugins_dir = std::env::var("RUSTMC_PLUGINS").unwrap_or_else(|_| "plugins".to_string());
    match bridge.load_plugins(&plugins_dir) {
        Ok(count) => tracing::info!("Loaded {count} plugin(s)"),
        Err(e) => tracing::warn!("Plugin loading failed: {e}"),
    }

    let server = network::Server::new(addr);
    server.run().await
}
