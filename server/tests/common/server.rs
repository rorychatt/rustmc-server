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
        let port = find_free_port().await?;

        let ops_file = if let Some(content) = ops_content {
            let path = std::env::temp_dir().join(format!("rustmc_ops_{port}.toml"));
            std::fs::write(&path, content)?;
            Some(path)
        } else {
            None
        };

        let mut cmd = Command::new("cargo");
        cmd.args(["run", "--bin", "rustmc-server"])
            .env("RUSTMC_BIND", format!("127.0.0.1:{port}"))
            .env("RUSTMC_PLUGINS", "")
            .env("RUST_LOG", "rustmc_server=warn")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        if let Some(ref path) = ops_file {
            cmd.env("RUSTMC_OPS", path.as_os_str());
        }

        let mut child = cmd.spawn()?;

        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(10) {
                let _ = child.kill();
                return Err(anyhow::anyhow!("Server failed to start within 10 seconds"));
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
