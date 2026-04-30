pub mod events;
pub mod java_plugin;
pub mod jvm;
pub mod plugin;
pub mod scheduler;

use std::sync::Arc;
use tracing::info;

pub use events::{Event, EventBus, EventHandler, EventPriority};
pub use java_plugin::JavaPlugin;
pub use jvm::JvmManager;
pub use plugin::{Plugin, PluginManager, PluginMeta};
pub use scheduler::{ScheduledTask, Scheduler, TaskHandle};

pub struct PluginBridge {
    pub event_bus: Arc<EventBus>,
    pub plugin_manager: PluginManager,
    pub scheduler: Scheduler,
}

impl PluginBridge {
    pub fn new() -> Self {
        let event_bus = Arc::new(EventBus::new());
        Self {
            event_bus: event_bus.clone(),
            plugin_manager: PluginManager::new(event_bus),
            scheduler: Scheduler::new(),
        }
    }

    pub fn load_plugins(&mut self, plugin_dir: &str) -> anyhow::Result<usize> {
        info!("Loading plugins from: {}", plugin_dir);
        self.plugin_manager.discover_and_load(plugin_dir)
    }

    pub async fn tick(&self) {
        self.scheduler.tick().await;
    }
}

impl Default for PluginBridge {
    fn default() -> Self {
        Self::new()
    }
}
