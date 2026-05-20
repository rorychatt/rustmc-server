use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;

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
            let path = std::env::temp_dir().join(format!("rustmc_ops_{}.toml", std::process::id()));
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
            let path = std::env::temp_dir().join(format!("rustmc_ops_{}.toml", std::process::id()));
            std::fs::write(&path, content)?;
            Some(path)
        } else {
            None
        };

        let mut all_env: Vec<(&str, String)> = extra_env
            .iter()
            .map(|(k, v)| (*k, v.to_string()))
            .collect();

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
        let port = find_free_port().await?;

        let build_status = Command::new("cargo")
            .args(["build", "--bin", "rustmc-server"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .status()?;

        if !build_status.success() {
            return Err(anyhow::anyhow!("Failed to build rustmc-server binary"));
        }

        let timeout_secs: u64 = std::env::var("RUSTMC_TEST_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);

        let mut cmd = Command::new("cargo");
        cmd.args(["run", "--bin", "rustmc-server"])
            .env("RUSTMC_BIND", format!("127.0.0.1:{port}"))
            .env("RUSTMC_PLUGINS", "")
            .env("RUST_LOG", "rustmc_server=warn")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        for (key, value) in extra_env {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn()?;

        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(timeout_secs) {
                let _ = child.kill();
                return Err(anyhow::anyhow!(
                    "Server failed to start within {timeout_secs} seconds"
                ));
            }

            if TcpListener::bind(format!("127.0.0.1:{port}"))
                .await
                .is_err()
            {
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        sleep(Duration::from_millis(500)).await;

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

async fn find_free_port() -> anyhow::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}
