use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Manages the lifecycle of a nostr-rs-relay instance for testing
pub struct RelayManager {
    process: Option<Child>,
    port: u16,
    #[allow(dead_code)]
    data_dir: Option<tempfile::TempDir>,
    #[allow(dead_code)]
    config_path: PathBuf,
}

impl RelayManager {
    /// Start a new relay instance or use existing one if available
    pub async fn start(verbose: bool) -> Result<Self> {
        let port = 7878;
        debug!("Checking if relay is already running on port {}", port);

        // Check if relay is already running
        if Self::is_port_open(port).await {
            info!("Found existing relay on port {}", port);
            if verbose {
                println!("  ℹ️  Using existing relay on port {port}");
            }
            return Ok(Self {
                process: None,
                port,
                data_dir: None,
                config_path: PathBuf::from("./gitsmith-integration-tests/relay-config.toml"),
            });
        }

        info!("No existing relay found, starting new instance");

        // Create temp directory for relay data
        let data_dir =
            tempfile::tempdir().context("Failed to create temporary directory for relay data")?;
        debug!("Created temp directory at {:?}", data_dir.path());

        // Find config file (handle different working directories)
        let config_path = if PathBuf::from("relay-config.toml").exists() {
            std::fs::canonicalize("relay-config.toml")
                .context("Failed to resolve relay-config.toml path")?
        } else if PathBuf::from("gitsmith-integration-tests/relay-config.toml").exists() {
            std::fs::canonicalize("gitsmith-integration-tests/relay-config.toml")
                .context("Failed to resolve gitsmith-integration-tests/relay-config.toml path")?
        } else {
            anyhow::bail!(
                "Could not find relay-config.toml in current directory or gitsmith-integration-tests/"
            );
        };
        debug!("Using config file: {:?}", config_path);

        if verbose || std::env::var("CI").is_ok() {
            println!("  🚀 Starting nostr-rs-relay on port {port}...");
            println!("     Config: {}", config_path.display());
            println!("     Data: {}", data_dir.path().display());
            if std::env::var("CI").is_ok() {
                println!("     Running in CI environment - using extended timeout");
            }
        }

        // Create database directory inside temp dir
        let db_dir = data_dir.path().join("test-relay-data");
        std::fs::create_dir_all(&db_dir).context("Failed to create database directory")?;

        // Verify relay binary exists
        if let Err(e) = std::process::Command::new("which")
            .arg("nostr-rs-relay")
            .output()
        {
            warn!("Failed to locate nostr-rs-relay binary: {}", e);
        }

        // Start nostr-rs-relay using tokio
        let mut cmd = Command::new("nostr-rs-relay");
        cmd.arg("--config")
            .arg(&config_path)
            .env("RUST_LOG", "warn") // Always use warn to avoid too much output
            .current_dir(data_dir.path())
            .stdout(std::process::Stdio::null()) // Always suppress stdout
            .stderr(std::process::Stdio::piped()) // Capture stderr for error reporting
            .kill_on_drop(true); // Ensure process is killed when dropped

        let mut process = cmd
            .spawn()
            .context("Failed to start nostr-rs-relay. Make sure it's installed (nix develop)")?;
        info!("Started nostr-rs-relay process");

        // Wait for relay to be ready
        if verbose {
            print!("     Waiting for relay to be ready");
        }
        debug!("Waiting for relay to be ready on port {}", port);

        // Try to wait for ready, capturing stderr on failure
        match Self::wait_for_ready(port, verbose).await {
            Ok(()) => {
                // Success - consume stderr to avoid broken pipe
                if let Some(stderr) = process.stderr.take() {
                    tokio::spawn(async move {
                        let mut stderr = stderr;
                        let mut buffer = Vec::new();
                        let _ = stderr.read_to_end(&mut buffer).await;
                    });
                }
            }
            Err(e) => {
                // Capture and log stderr on failure
                if let Some(mut stderr) = process.stderr.take() {
                    let mut buffer = Vec::new();
                    let _ = stderr.read_to_end(&mut buffer).await;
                    let stderr_output = String::from_utf8_lossy(&buffer);
                    if !stderr_output.is_empty() {
                        warn!("Relay stderr output:\n{}", stderr_output);
                        eprintln!("❌ Relay failed to start. Error output:");
                        eprintln!("{}", stderr_output);
                    }
                }
                // Kill the process before returning error
                let _ = process.kill().await;
                return Err(e.context("Relay failed to become ready"));
            }
        }
        info!("Relay is ready and accepting connections");
        if verbose {
            println!(" ✓");
        }

        Ok(Self {
            process: Some(process),
            port,
            data_dir: Some(data_dir),
            config_path,
        })
    }

    /// Check if a port is open (TCP connection test)
    async fn is_port_open(port: u16) -> bool {
        tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok()
    }

    /// Wait for the relay to be ready to accept connections
    async fn wait_for_ready(port: u16, verbose: bool) -> Result<()> {
        // CI environments may be slower, so use a longer timeout
        let timeout_seconds = if std::env::var("CI").is_ok() { 60 } else { 30 };

        for i in 0..timeout_seconds {
            if Self::is_port_open(port).await {
                return Ok(());
            }
            if verbose && i > 0 && i % 5 == 0 {
                print!(".");
            }
            // Use exponential backoff for the first few attempts
            let delay = if i < 5 {
                Duration::from_millis(100 * (2_u64.pow(i as u32)))
            } else {
                Duration::from_secs(1)
            };
            sleep(delay).await;
        }
        anyhow::bail!("Relay failed to start within {} seconds", timeout_seconds)
    }

    /// Get the WebSocket URL for the relay
    pub fn get_url(&self) -> String {
        format!("ws://localhost:{}", self.port)
    }
}

impl Drop for RelayManager {
    fn drop(&mut self) {
        // Only kill the process if we started it
        // Note: kill_on_drop(true) was set, so tokio will handle this automatically
        // We just need to drop the Child handle
        self.process = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_port_check() -> Result<()> {
        // Port 0 should never be open
        assert!(!RelayManager::is_port_open(0).await);
        Ok(())
    }

    #[test]
    fn test_get_url() {
        let manager = RelayManager {
            process: None,
            port: 7878,
            data_dir: None,
            config_path: PathBuf::from("test.toml"),
        };
        assert_eq!(manager.get_url(), "ws://localhost:7878");
    }
}
