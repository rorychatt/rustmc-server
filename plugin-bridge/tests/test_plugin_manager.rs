use std::sync::Arc;

use plugin_bridge::{EventBus, Plugin, PluginManager, PluginMeta};

struct MockPlugin {
    meta: PluginMeta,
}

impl Plugin for MockPlugin {
    fn meta(&self) -> &PluginMeta {
        &self.meta
    }

    fn on_enable(&self, _event_bus: &EventBus) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_disable(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[test]
fn test_discover_empty_directory() {
    let dir = tempfile::tempdir().unwrap();
    let event_bus = Arc::new(EventBus::new());
    let mut manager = PluginManager::new(event_bus);

    let count = manager.discover_and_load(dir.path().to_str().unwrap()).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_discover_nonexistent_directory_creates_it() {
    let dir = tempfile::tempdir().unwrap();
    let plugin_dir = dir.path().join("plugins");
    let event_bus = Arc::new(EventBus::new());
    let mut manager = PluginManager::new(event_bus);

    let count = manager
        .discover_and_load(plugin_dir.to_str().unwrap())
        .unwrap();
    assert_eq!(count, 0);
    assert!(plugin_dir.exists());
}

#[test]
fn test_register_and_list_plugins() {
    let event_bus = Arc::new(EventBus::new());
    let mut manager = PluginManager::new(event_bus);

    let plugin = Box::new(MockPlugin {
        meta: PluginMeta {
            name: "TestPlugin".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            main_class: "com.test.Test".to_string(),
        },
    });

    manager.register_plugin(plugin).unwrap();
    assert_eq!(manager.loaded_plugins().len(), 1);
    assert_eq!(manager.loaded_plugins()[0].name, "TestPlugin");
}

#[test]
fn test_disable_all_plugins() {
    let event_bus = Arc::new(EventBus::new());
    let mut manager = PluginManager::new(event_bus);

    for i in 0..3 {
        let plugin = Box::new(MockPlugin {
            meta: PluginMeta {
                name: format!("Plugin{i}"),
                version: "1.0.0".to_string(),
                description: String::new(),
                main_class: format!("com.test.Plugin{i}"),
            },
        });
        manager.register_plugin(plugin).unwrap();
    }

    assert_eq!(manager.loaded_plugins().len(), 3);
    manager.disable_all().unwrap();
    assert_eq!(manager.loaded_plugins().len(), 0);
}

#[test]
fn test_discover_with_non_jar_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("readme.txt"), "not a plugin").unwrap();
    std::fs::write(dir.path().join("config.yml"), "config: true").unwrap();

    let event_bus = Arc::new(EventBus::new());
    let mut manager = PluginManager::new(event_bus);

    let count = manager.discover_and_load(dir.path().to_str().unwrap()).unwrap();
    assert_eq!(count, 0);
}
