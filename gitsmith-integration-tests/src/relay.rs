use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::time::sleep;
use tracing::{debug, info};

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
            PathBuf::from("relay-config.toml")
        } else if PathBuf::from("gitsmith-integration-tests/relay-config.toml").exists() {
            PathBuf::from("gitsmith-integration-tests/relay-config.toml")
        } else {
            anyhow::bail!(
                "Could not find relay-config.toml in current directory or gitsmith-integration-tests/"
            );
        };
        debug!("Using config file: {:?}", config_path);

        if verbose {
            println!("  🚀 Starting nostr-rs-relay on port {port}...");
            println!("     Config: {}", config_path.display());
            println!("     Data: {}", data_dir.path().display());
        }

        // Create database directory inside temp dir
        let db_dir = data_dir.path().join("test-relay-data");
        std::fs::create_dir_all(&db_dir).context("Failed to create database directory")?;

        // Start nostr-rs-relay using tokio
        let mut cmd = Command::new("nostr-rs-relay");
        cmd.arg("--config")
            .arg(&config_path)
            .env("RUST_LOG", "warn") // Always use warn to avoid too much output
            .current_dir(data_dir.path())
            .stdout(std::process::Stdio::null()) // Always suppress stdout
            .stderr(if verbose {
                std::process::Stdio::inherit()
            } else {
                std::process::Stdio::null()
            })
            .kill_on_drop(true); // Ensure process is killed when dropped

        let process = cmd
            .spawn()
            .context("Failed to start nostr-rs-relay. Make sure it's installed (nix develop)")?;
        info!("Started nostr-rs-relay process");

        // Wait for relay to be ready
        if verbose {
            print!("     Waiting for relay to be ready");
        }
        debug!("Waiting for relay to be ready on port {}", port);
        Self::wait_for_ready(port, verbose).await?;
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
        for i in 0..30 {
            if Self::is_port_open(port).await {
                return Ok(());
            }
            if verbose && i > 0 && i % 5 == 0 {
                print!(".");
            }
            sleep(Duration::from_secs(1)).await;
        }
        anyhow::bail!("Relay failed to start within 30 seconds")
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
