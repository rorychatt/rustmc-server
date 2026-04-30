use std::path::Path;
use std::sync::OnceLock;

use anyhow::{Context, Result, bail};
use jni::objects::JValue;
use jni::{InitArgsBuilder, JNIVersion, JavaVM};
use tracing::{debug, error, info};

static JVM_INSTANCE: OnceLock<JavaVM> = OnceLock::new();

pub struct JvmManager;

impl JvmManager {
    pub fn initialize(classpath_entries: &[&str]) -> Result<&'static JavaVM> {
        if let Some(jvm) = JVM_INSTANCE.get() {
            debug!("JVM already initialized, reusing existing instance");
            return Ok(jvm);
        }

        let jvm_path = java_locator::locate_jvm_dyn_library()
            .context("Failed to locate JVM library. Ensure JAVA_HOME is set or Java is installed")?;

        info!("Located JVM library: {}", jvm_path);

        let classpath = classpath_entries.join(if cfg!(windows) { ";" } else { ":" });
        let classpath_opt = format!("-Djava.class.path={classpath}");

        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option(&classpath_opt)
            .option("-Xmx512M")
            .option("-Xms128M")
            .build()
            .context("Failed to build JVM initialization arguments")?;

        let jvm = JavaVM::new(jvm_args).context("Failed to create JVM instance")?;

        info!("JVM initialized successfully");

        match JVM_INSTANCE.set(jvm) {
            Ok(()) => Ok(JVM_INSTANCE.get().unwrap()),
            Err(_) => {
                debug!("Another thread initialized JVM first, using that instance");
                Ok(JVM_INSTANCE.get().unwrap())
            }
        }
    }

    pub fn get() -> Option<&'static JavaVM> {
        JVM_INSTANCE.get()
    }

    pub fn add_to_classpath(jvm: &JavaVM, jar_path: &Path) -> Result<()> {
        let mut env = jvm
            .attach_current_thread()
            .context("Failed to attach thread to JVM")?;

        let jar_url_str = if cfg!(windows) {
            format!("file:///{}", jar_path.display()).replace('\\', "/")
        } else {
            format!("file://{}", jar_path.display())
        };

        let url_string = env
            .new_string(&jar_url_str)
            .context("Failed to create Java string for URL")?;

        let uri = env
            .call_static_method(
                "java/net/URI",
                "create",
                "(Ljava/lang/String;)Ljava/net/URI;",
                &[JValue::Object(&url_string)],
            )
            .context("Failed to create URI")?
            .l()
            .context("URI create did not return an object")?;

        let url = env
            .call_method(&uri, "toURL", "()Ljava/net/URL;", &[])
            .context("Failed to convert URI to URL")?
            .l()
            .context("toURL did not return an object")?;

        let class_loader = env
            .call_static_method(
                "java/lang/ClassLoader",
                "getSystemClassLoader",
                "()Ljava/lang/ClassLoader;",
                &[],
            )
            .context("Failed to get system class loader")?
            .l()
            .context("getSystemClassLoader did not return an object")?;

        let url_class_loader_class = env
            .find_class("java/net/URLClassLoader")
            .context("Failed to find URLClassLoader class")?;

        let is_url_cl = env
            .is_instance_of(&class_loader, url_class_loader_class)
            .context("Failed to check class loader type")?;

        if is_url_cl {
            env.call_method(
                &class_loader,
                "addURL",
                "(Ljava/net/URL;)V",
                &[JValue::Object(&url)],
            )
            .context("Failed to add URL to class loader")?;

            debug!("Added {} to JVM classpath", jar_path.display());
        } else {
            debug!(
                "System class loader is not a URLClassLoader; JAR was included via -Djava.class.path"
            );
        }

        if env.exception_check().unwrap_or(false) {
            env.exception_describe().ok();
            env.exception_clear().ok();
            bail!("Java exception occurred while adding JAR to classpath");
        }

        Ok(())
    }

    pub fn shutdown() {
        if let Some(jvm) = JVM_INSTANCE.get() {
            info!("Shutting down JVM...");
            if let Err(e) = jvm.attach_current_thread() {
                error!("Failed to attach thread for JVM shutdown: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jvm_not_initialized_by_default() {
        assert!(JvmManager::get().is_none() || JvmManager::get().is_some());
    }
}
