use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Duration;
use tokio::time::sleep;

static FILE_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub struct TestServer {
    process: Child,
    port: u16,
    _ops_file: Option<PathBuf>,
}

impl TestServer {
    pub async fn spawn() -> anyhow::Result<Self> {
        Self::spawn_with_ops(None).await
    }

    pub async fn spawn_with_ops(ops_content: Option<&str>) -> anyhow::Result<Self> {
        let ops_file = if let Some(content) = ops_content {
            let count = FILE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "rustmc_ops_{}_{:?}_{count}.toml",
                std::process::id(),
                std::thread::current().id()
            ));
            std::fs::write(&path, content)?;
            Some(path)
        } else {
            None
        };

        let extra_env: Vec<(&str, String)> = if let Some(ref path) = ops_file {
            vec![("RUSTMC_OPS", path.to_string_lossy().into_owned())]
        } else {
            vec![]
        };

        let extra_refs: Vec<(&str, &str)> =
            extra_env.iter().map(|(k, v)| (*k, v.as_str())).collect();
        Self::spawn_with_env_and_ops(&extra_refs, ops_file).await
    }

    #[allow(dead_code)]
    pub async fn spawn_with_env(extra_env: &[(&str, &str)]) -> anyhow::Result<Self> {
        Self::spawn_with_env_and_ops(extra_env, None).await
    }

    #[allow(dead_code)]
    pub async fn spawn_with_env_and_ops_config(
        extra_env: &[(&str, &str)],
        ops_content: Option<&str>,
    ) -> anyhow::Result<Self> {
        let ops_file = if let Some(content) = ops_content {
            let count = FILE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "rustmc_ops_{}_{:?}_{count}.toml",
                std::process::id(),
                std::thread::current().id()
            ));
            std::fs::write(&path, content)?;
            Some(path)
        } else {
            None
        };

        let mut all_env: Vec<(&str, String)> =
            extra_env.iter().map(|(k, v)| (*k, v.to_string())).collect();

        if let Some(ref path) = ops_file {
            all_env.push(("RUSTMC_OPS", path.to_string_lossy().into_owned()));
        }

        let all_refs: Vec<(&str, &str)> = all_env.iter().map(|(k, v)| (*k, v.as_str())).collect();
        Self::spawn_with_env_and_ops(&all_refs, ops_file).await
    }

    async fn spawn_with_env_and_ops(
        extra_env: &[(&str, &str)],
        ops_file: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let count = FILE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let port_file = std::env::temp_dir().join(format!(
            "rustmc_port_{}_{}_{count}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));

        let timeout_secs: u64 = std::env::var("RUSTMC_TEST_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);

        let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustmc-server"));
        cmd.env("RUSTMC_BIND", "127.0.0.1:0")
            .env("RUSTMC_PORT_FILE", &port_file)
            .env("RUSTMC_PLUGINS", "")
            .env("RUST_LOG", "rustmc_server=warn")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        for (key, value) in extra_env {
            cmd.env(key, value);
        }

        let child = cmd.spawn()?;

        let start = std::time::Instant::now();
        let port = loop {
            if start.elapsed() > Duration::from_secs(timeout_secs) {
                let _ = std::fs::remove_file(&port_file);
                return Err(anyhow::anyhow!(
                    "Server failed to start within {timeout_secs} seconds"
                ));
            }

            if let Ok(contents) = std::fs::read_to_string(&port_file) {
                if let Ok(p) = contents.trim().parse::<u16>() {
                    break p;
                }
            }

            sleep(Duration::from_millis(50)).await;
        };

        let _ = std::fs::remove_file(&port_file);

        Ok(TestServer {
            process: child,
            port,
            _ops_file: ops_file,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
        if let Some(ref path) = self._ops_file {
            let _ = std::fs::remove_file(path);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
