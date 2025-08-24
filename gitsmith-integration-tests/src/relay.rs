use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Type of relay to start
#[derive(Debug, Clone)]
pub enum RelayType {
    NostrRsRelay,
    Strfry,
}

/// Manages the lifecycle of a relay instance for testing
pub struct RelayManager {
    process: Option<Child>,
    port: u16,
    relay_type: RelayType,
    #[allow(dead_code)]
    data_dir: Option<tempfile::TempDir>,
    #[allow(dead_code)]
    config_path: PathBuf,
}

impl RelayManager {
    /// Start a single relay (backward compatibility)
    pub async fn start() -> Result<Self> {
        Self::start_nostr_rs_relay(7878).await
    }

    /// Start multiple relays of different types
    pub async fn start_multiple() -> Result<Vec<Self>> {
        let mut managers = Vec::new();

        // Start nostr-rs-relay on port 7878
        managers.push(Self::start_nostr_rs_relay(7878).await?);

        // Try to start strfry on port 7879, fall back to second nostr-rs-relay if strfry unavailable
        match Self::start_strfry(7879).await {
            Ok(manager) => managers.push(manager),
            Err(e) => {
                warn!(
                    "Failed to start strfry: {}. Falling back to second nostr-rs-relay",
                    e
                );
                managers.push(Self::start_nostr_rs_relay(7879).await?);
            }
        }

        Ok(managers)
    }

    /// Start a new nostr-rs-relay instance or use existing one if available
    pub async fn start_nostr_rs_relay(port: u16) -> Result<Self> {
        debug!(
            "Checking if nostr-rs-relay is already running on port {}",
            port
        );

        // Check if relay is already running
        if Self::is_port_open(port).await {
            info!("Found existing nostr-rs-relay on port {}", port);
            println!("  ℹ️  Using existing nostr-rs-relay on port {port}");
            return Ok(Self {
                process: None,
                port,
                relay_type: RelayType::NostrRsRelay,
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
        let config_name = if port == 7878 {
            "relay-config.toml"
        } else {
            "relay-config-7879.toml"
        };

        let config_path = if PathBuf::from(config_name).exists() {
            std::fs::canonicalize(config_name)
                .with_context(|| format!("Failed to resolve {} path", config_name))?
        } else if PathBuf::from(format!("gitsmith-integration-tests/{}", config_name)).exists() {
            std::fs::canonicalize(format!("gitsmith-integration-tests/{}", config_name))
                .with_context(|| {
                    format!(
                        "Failed to resolve gitsmith-integration-tests/{} path",
                        config_name
                    )
                })?
        } else {
            anyhow::bail!(
                "Could not find {} in current directory or gitsmith-integration-tests/",
                config_name
            );
        };
        debug!("Using config file: {:?}", config_path);

        // Always show relay setup information
        println!("  🚀 Starting nostr-rs-relay on port {port}...");
        println!("     Config: {}", config_path.display());
        println!("     Data: {}", data_dir.path().display());
        if std::env::var("CI").is_ok() {
            println!("     Running in CI environment - using extended timeout");
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
        print!("     Waiting for relay to be ready");
        debug!("Waiting for relay to be ready on port {}", port);

        // Try to wait for ready, capturing stderr on failure
        match Self::wait_for_ready(port).await {
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
        println!(" ✓");

        Ok(Self {
            process: Some(process),
            port,
            relay_type: RelayType::NostrRsRelay,
            data_dir: Some(data_dir),
            config_path,
        })
    }

    /// Start a new strfry instance
    pub async fn start_strfry(port: u16) -> Result<Self> {
        debug!("Checking if strfry is already running on port {}", port);

        // Check if relay is already running
        if Self::is_port_open(port).await {
            info!("Found existing strfry on port {}", port);
            println!("  ℹ️  Using existing strfry on port {port}");
            return Ok(Self {
                process: None,
                port,
                relay_type: RelayType::Strfry,
                data_dir: None,
                config_path: PathBuf::from("./gitsmith-integration-tests/strfry-config.conf"),
            });
        }

        info!("No existing strfry found, starting new instance");

        // Create temp directory for strfry data
        let data_dir =
            tempfile::tempdir().context("Failed to create temporary directory for strfry data")?;
        debug!("Created temp directory at {:?}", data_dir.path());

        // Find config file (handle different working directories)
        let config_name = "strfry-config.conf";
        let config_path = if PathBuf::from(config_name).exists() {
            std::fs::canonicalize(config_name)
                .with_context(|| format!("Failed to resolve {} path", config_name))?
        } else if PathBuf::from(format!("gitsmith-integration-tests/{}", config_name)).exists() {
            std::fs::canonicalize(format!("gitsmith-integration-tests/{}", config_name))
                .with_context(|| {
                    format!(
                        "Failed to resolve gitsmith-integration-tests/{} path",
                        config_name
                    )
                })?
        } else {
            anyhow::bail!(
                "Could not find {} in current directory or gitsmith-integration-tests/",
                config_name
            );
        };
        debug!("Using config file: {:?}", config_path);

        // Always show relay setup information
        println!("  🚀 Starting strfry on port {port}...");
        println!("     Config: {}", config_path.display());
        println!("     Data: {}", data_dir.path().display());
        if std::env::var("CI").is_ok() {
            println!("     Running in CI environment - using extended timeout");
        }

        // Create database directory inside temp dir
        let db_dir = data_dir.path().join("strfry-db");
        std::fs::create_dir_all(&db_dir).context("Failed to create strfry database directory")?;

        // Verify strfry binary exists
        if let Err(e) = std::process::Command::new("which").arg("strfry").output() {
            warn!("Failed to locate strfry binary: {}", e);
            anyhow::bail!("strfry not found. Make sure it's installed (nix develop)");
        }

        // Start strfry using tokio
        let mut cmd = Command::new("strfry");
        cmd.arg(format!("--config={}", config_path.display()))
            .arg("relay")
            .env("RUST_LOG", "warn")
            .current_dir(data_dir.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let mut process = cmd
            .spawn()
            .context("Failed to start strfry. Make sure it's installed (nix develop)")?;
        info!("Started strfry process");

        // Wait for relay to be ready
        print!("     Waiting for strfry to be ready");
        debug!("Waiting for strfry to be ready on port {}", port);

        // Try to wait for ready, capturing stderr on failure
        match Self::wait_for_ready(port).await {
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
                        warn!("Strfry stderr output:\n{}", stderr_output);
                        eprintln!("❌ Strfry failed to start. Error output:");
                        eprintln!("{}", stderr_output);
                    }
                }
                // Kill the process before returning error
                let _ = process.kill().await;
                return Err(e.context("Strfry failed to become ready"));
            }
        }
        info!("Strfry is ready and accepting connections");
        println!(" ✓");

        Ok(Self {
            process: Some(process),
            port,
            relay_type: RelayType::Strfry,
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
    async fn wait_for_ready(port: u16) -> Result<()> {
        // CI environments may be slower, so use a longer timeout
        let timeout_seconds = if std::env::var("CI").is_ok() { 60 } else { 30 };

        for i in 0..timeout_seconds {
            if Self::is_port_open(port).await {
                return Ok(());
            }
            if i > 0 && i % 5 == 0 {
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
            relay_type: RelayType::NostrRsRelay,
            data_dir: None,
            config_path: PathBuf::from("test.toml"),
        };
        assert_eq!(manager.get_url(), "ws://localhost:7878");
    }
}
