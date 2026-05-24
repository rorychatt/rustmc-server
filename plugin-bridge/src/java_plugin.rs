use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use jni::objects::GlobalRef;
use jni::JavaVM;
use tracing::{debug, info, warn};

use super::events::EventBus;
use super::jvm::JvmManager;
use super::plugin::{Plugin, PluginMeta};

pub struct JavaPlugin {
    jvm: &'static JavaVM,
    meta: PluginMeta,
    plugin_instance: Option<GlobalRef>,
    jar_path: PathBuf,
}

impl JavaPlugin {
    pub fn new(jvm: &'static JavaVM, jar_path: &Path) -> Result<Self> {
        let meta = Self::parse_plugin_yml(jar_path)
            .with_context(|| format!("Failed to parse plugin.yml from {}", jar_path.display()))?;

        info!(
            "Loading Java plugin: {} v{} ({})",
            meta.name, meta.version, meta.main_class
        );

        JvmManager::add_to_classpath(jvm, jar_path)
            .with_context(|| format!("Failed to add {} to classpath", jar_path.display()))?;

        let mut env = jvm
            .attach_current_thread()
            .context("Failed to attach thread to JVM")?;

        let class_name = meta.main_class.replace('.', "/");
        let plugin_class = env
            .find_class(&class_name)
            .with_context(|| format!("Failed to find main class: {}", meta.main_class))?;

        if env.exception_check().unwrap_or(false) {
            env.exception_describe().ok();
            env.exception_clear().ok();
            bail!("Java exception while loading class: {}", meta.main_class);
        }

        let instance = env
            .new_object(plugin_class, "()V", &[])
            .with_context(|| format!("Failed to instantiate plugin class: {}", meta.main_class))?;

        if env.exception_check().unwrap_or(false) {
            env.exception_describe().ok();
            env.exception_clear().ok();
            bail!(
                "Java exception during plugin construction: {}",
                meta.main_class
            );
        }

        let global_ref = env
            .new_global_ref(instance)
            .context("Failed to create global reference to plugin instance")?;

        debug!("Plugin {} instantiated successfully", meta.name);

        Ok(Self {
            jvm,
            meta,
            plugin_instance: Some(global_ref),
            jar_path: jar_path.to_path_buf(),
        })
    }

    pub fn new_from_jar_meta(jar_path: &Path) -> Result<PluginMeta> {
        Self::parse_plugin_yml(jar_path)
    }

    fn parse_plugin_yml(jar_path: &Path) -> Result<PluginMeta> {
        let jar_str = jar_path.to_string_lossy();
        if jar_str.contains("..") || jar_path.components().any(|c| c == std::path::Component::ParentDir) {
            bail!("Path traversal detected in JAR path: {}", jar_path.display());
        }
        let canonical_path = jar_path.canonicalize()
            .with_context(|| format!("Failed to canonicalize JAR path: {}", jar_path.display()))?;
        if canonical_path.components().any(|c| c == std::path::Component::ParentDir) {
            bail!("Path traversal detected in canonicalized JAR path: {}", canonical_path.display());
        }
        let file = std::fs::File::open(&canonical_path)
            .with_context(|| format!("Failed to open JAR: {}", canonical_path.display()))?;

        let mut archive = zip::ZipArchive::new(file)
            .with_context(|| format!("Failed to read JAR as ZIP: {}", jar_path.display()))?;

        let plugin_yml_name = if archive.by_name("plugin.yml").is_ok() {
            "plugin.yml"
        } else if archive.by_name("paper-plugin.yml").is_ok() {
            "paper-plugin.yml"
        } else {
            bail!(
                "No plugin.yml or paper-plugin.yml found in {}",
                jar_path.display()
            );
        };

        let mut yml_file = archive
            .by_name(plugin_yml_name)
            .with_context(|| format!("Failed to read {plugin_yml_name}"))?;

        let mut contents = String::new();
        yml_file
            .read_to_string(&mut contents)
            .context("Failed to read plugin.yml contents")?;

        let name = Self::extract_yml_value(&contents, "name")
            .unwrap_or_else(|| "UnknownPlugin".to_string());
        let version =
            Self::extract_yml_value(&contents, "version").unwrap_or_else(|| "0.0.0".to_string());
        let description = Self::extract_yml_value(&contents, "description").unwrap_or_default();
        let main_class = Self::extract_yml_value(&contents, "main")
            .context("plugin.yml is missing required 'main' field")?;

        Ok(PluginMeta {
            name,
            version,
            description,
            main_class,
        })
    }

    fn extract_yml_value(contents: &str, key: &str) -> Option<String> {
        for line in contents.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix(key) {
                let rest = rest.trim_start();
                if let Some(value) = rest.strip_prefix(':') {
                    let value = value.trim();
                    let value = value.trim_matches('"').trim_matches('\'');
                    if !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
        }
        None
    }

    fn call_java_method(&self, method_name: &str) -> Result<()> {
        let instance = self
            .plugin_instance
            .as_ref()
            .context("Plugin instance has been dropped")?;

        let mut env = self
            .jvm
            .attach_current_thread()
            .context("Failed to attach thread to JVM")?;

        env.call_method(instance.as_obj(), method_name, "()V", &[])
            .with_context(|| {
                format!(
                    "Failed to call {}() on plugin {}",
                    method_name, self.meta.name
                )
            })?;

        if env.exception_check().unwrap_or(false) {
            env.exception_describe().ok();
            env.exception_clear().ok();
            bail!("Java exception in {}.{}()", self.meta.name, method_name);
        }

        Ok(())
    }

    pub fn jar_path(&self) -> &Path {
        &self.jar_path
    }
}

impl Plugin for JavaPlugin {
    fn meta(&self) -> &PluginMeta {
        &self.meta
    }

    fn on_enable(&self, _event_bus: &EventBus) -> Result<()> {
        info!("Enabling Java plugin: {}", self.meta.name);
        self.call_java_method("onEnable")
    }

    fn on_disable(&self) -> Result<()> {
        info!("Disabling Java plugin: {}", self.meta.name);
        match self.call_java_method("onDisable") {
            Ok(()) => {}
            Err(e) => {
                warn!("Error calling onDisable for plugin {}: {e}", self.meta.name);
            }
        }
        Ok(())
    }
}

impl Drop for JavaPlugin {
    fn drop(&mut self) {
        if let Some(global_ref) = self.plugin_instance.take() {
            debug!("Dropping global reference for plugin: {}", self.meta.name);
            drop(global_ref);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_yml_value() {
        let yml = r#"
name: TestPlugin
version: "1.0.0"
main: com.example.TestPlugin
description: 'A test plugin'
"#;
        assert_eq!(
            JavaPlugin::extract_yml_value(yml, "name"),
            Some("TestPlugin".to_string())
        );
        assert_eq!(
            JavaPlugin::extract_yml_value(yml, "version"),
            Some("1.0.0".to_string())
        );
        assert_eq!(
            JavaPlugin::extract_yml_value(yml, "main"),
            Some("com.example.TestPlugin".to_string())
        );
        assert_eq!(
            JavaPlugin::extract_yml_value(yml, "description"),
            Some("A test plugin".to_string())
        );
        assert_eq!(JavaPlugin::extract_yml_value(yml, "missing"), None);
    }

    #[test]
    fn test_extract_yml_value_no_quotes() {
        let yml = "name: SimplePlugin\nversion: 2.0\nmain: org.example.Main\n";
        assert_eq!(
            JavaPlugin::extract_yml_value(yml, "name"),
            Some("SimplePlugin".to_string())
        );
        assert_eq!(
            JavaPlugin::extract_yml_value(yml, "version"),
            Some("2.0".to_string())
        );
    }
}
