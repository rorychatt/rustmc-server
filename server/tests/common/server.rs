use std::process::{Child, Command};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;

pub struct TestServer {
    process: Child,
    port: u16,
}

impl TestServer {
    pub async fn spawn() -> anyhow::Result<Self> {
        let port = find_free_port().await?;

        let mut child = Command::new("cargo")
            .args(["run", "--bin", "rustmc-server"])
            .env("RUSTMC_BIND", format!("127.0.0.1:{port}"))
            .env("RUSTMC_PLUGINS", "")
            .env("RUST_LOG", "rustmc_server=warn")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        // Wait for server to be ready
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
                // Port is in use, server is likely ready
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        // Give it a bit more time to fully initialize
        sleep(Duration::from_millis(500)).await;

        Ok(TestServer {
            process: child,
            port,
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
        // Give OS time to free the port
        std::thread::sleep(Duration::from_millis(100));
    }
}

async fn find_free_port() -> anyhow::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}
