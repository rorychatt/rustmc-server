use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use jni::JavaVM;
use tracing::{error, info, warn};

use super::events::EventBus;
use super::java_plugin::JavaPlugin;
use super::jvm::JvmManager;

#[derive(Debug, Clone)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
    pub main_class: String,
}

pub trait Plugin: Send + Sync {
    fn meta(&self) -> &PluginMeta;
    fn on_enable(&self, event_bus: &EventBus) -> Result<()>;
    fn on_disable(&self) -> Result<()>;
}

pub struct PluginManager {
    plugins: HashMap<String, Box<dyn Plugin>>,
    event_bus: Arc<EventBus>,
    jvm: Option<&'static JavaVM>,
}

impl PluginManager {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            plugins: HashMap::new(),
            event_bus,
            jvm: None,
        }
    }

    fn ensure_jvm(&mut self, classpath_entries: &[&str]) -> Result<&'static JavaVM> {
        if let Some(jvm) = self.jvm {
            return Ok(jvm);
        }
        let jvm = JvmManager::initialize(classpath_entries)?;
        self.jvm = Some(jvm);
        Ok(jvm)
    }

    pub fn discover_and_load(&mut self, plugin_dir: &str) -> Result<usize> {
        let path = Path::new(plugin_dir);
        if !path.exists() {
            info!("Plugin directory does not exist: {plugin_dir}, creating it");
            std::fs::create_dir_all(path)?;
            return Ok(0);
        }

        let mut jar_paths = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let file_path = entry.path();
            if file_path.extension().and_then(|e| e.to_str()) == Some("jar") {
                info!("Found plugin JAR: {}", file_path.display());
                jar_paths.push(file_path);
            }
        }

        if jar_paths.is_empty() {
            info!("No plugin JARs found in {plugin_dir}");
            return Ok(0);
        }

        let classpath_strs: Vec<String> = jar_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        let classpath_refs: Vec<&str> = classpath_strs.iter().map(|s| s.as_str()).collect();

        let jvm = match self.ensure_jvm(&classpath_refs) {
            Ok(jvm) => jvm,
            Err(e) => {
                error!("Failed to initialize JVM: {e}");
                warn!("Falling back to discovery-only mode — Java plugins will not be loaded");
                info!("Discovered {} plugin JAR(s) in {plugin_dir} (JVM unavailable)", jar_paths.len());
                return Ok(jar_paths.len());
            }
        };

        let mut count = 0;
        for jar_path in &jar_paths {
            match JavaPlugin::new(jvm, jar_path) {
                Ok(plugin) => {
                    let plugin_box: Box<dyn Plugin> = Box::new(plugin);
                    match self.register_plugin(plugin_box) {
                        Ok(()) => count += 1,
                        Err(e) => {
                            error!(
                                "Failed to enable plugin from {}: {e}",
                                jar_path.display()
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Skipping malformed plugin {}: {e}",
                        jar_path.display()
                    );
                }
            }
        }

        info!("Loaded {count}/{} plugin(s) from {plugin_dir}", jar_paths.len());
        Ok(count)
    }

    pub fn register_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<()> {
        let name = plugin.meta().name.clone();
        info!("Enabling plugin: {name} v{}", plugin.meta().version);
        plugin.on_enable(&self.event_bus)?;
        self.plugins.insert(name, plugin);
        Ok(())
    }

    pub fn disable_plugin(&mut self, name: &str) -> Result<()> {
        if let Some(plugin) = self.plugins.remove(name) {
            info!("Disabling plugin: {name}");
            plugin.on_disable()?;
            self.event_bus.unregister_plugin(name);
        }
        Ok(())
    }

    pub fn disable_all(&mut self) -> Result<()> {
        let names: Vec<_> = self.plugins.keys().cloned().collect();
        for name in names {
            self.disable_plugin(&name)?;
        }
        if self.jvm.is_some() {
            JvmManager::shutdown();
            self.jvm = None;
        }
        Ok(())
    }

    pub fn loaded_plugins(&self) -> Vec<&PluginMeta> {
        self.plugins.values().map(|p| p.meta()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin {
        meta: PluginMeta,
    }

    impl Plugin for TestPlugin {
        fn meta(&self) -> &PluginMeta {
            &self.meta
        }

        fn on_enable(&self, _event_bus: &EventBus) -> Result<()> {
            Ok(())
        }

        fn on_disable(&self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_register_and_disable_plugin() {
        let event_bus = Arc::new(EventBus::new());
        let mut manager = PluginManager::new(event_bus);

        let plugin = Box::new(TestPlugin {
            meta: PluginMeta {
                name: "TestPlugin".to_string(),
                version: "1.0.0".to_string(),
                description: "A test plugin".to_string(),
                main_class: "com.test.TestPlugin".to_string(),
            },
        });

        manager.register_plugin(plugin).unwrap();
        assert_eq!(manager.loaded_plugins().len(), 1);

        manager.disable_plugin("TestPlugin").unwrap();
        assert_eq!(manager.loaded_plugins().len(), 0);
    }
}
