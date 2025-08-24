use anyhow::{Context, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

/// Runner for gitsmith commands
pub struct GitsmithRunner {
    home_dir: String,
}

impl GitsmithRunner {
    /// Create a new gitsmith runner with HOME environment set
    pub fn new(home_dir: &Path) -> Self {
        Self {
            home_dir: home_dir.to_string_lossy().to_string(),
        }
    }

    /// Run a gitsmith command with arguments, piping output directly to stdout/stderr
    pub async fn run(&self, args: &[&str]) -> Result<CommandOutput> {
        println!("    $ gitsmith {}", args.join(" "));

        // Determine the path to the gitsmith binary
        let gitsmith_path = std::env::current_exe()
            .ok()
            .and_then(|p| {
                let parent = p.parent()?;
                let gitsmith = parent.join("gitsmith");
                if gitsmith.exists() {
                    Some(gitsmith)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                // Fallback to cargo run
                std::path::PathBuf::from("cargo")
            });

        let output = if gitsmith_path.to_string_lossy() == "cargo" {
            Command::new("cargo")
                .args([
                    "run",
                    "--manifest-path",
                    "/home/master/p/gitsmith/Cargo.toml",
                    "--bin",
                    "gitsmith",
                    "--",
                ])
                .args(args)
                .env("HOME", &self.home_dir)
                .env("GITSMITH_PASSWORD", "test")
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .output()
                .await?
        } else {
            Command::new(gitsmith_path)
                .args(args)
                .env("HOME", &self.home_dir)
                .env("GITSMITH_PASSWORD", "test")
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .output()
                .await?
        };

        // Capture stdout only (stderr goes directly to terminal)
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        // Print stdout for visibility
        if !stdout.is_empty() {
            print!("{}", stdout);
        }

        let result = CommandOutput {
            stdout,
            stderr: String::new(),
            success: output.status.success(),
            _exit_code: output.status.code().unwrap_or(-1),
        };

        Ok(result)
    }

    /// Run a gitsmith command with JSON output capture
    /// This version captures stdout for JSON parsing while still showing stderr
    pub async fn run_json(&self, args: &[&str]) -> Result<CommandOutput> {
        println!("    $ gitsmith {}", args.join(" "));

        // Determine the path to the gitsmith binary
        let gitsmith_path = std::env::current_exe()
            .ok()
            .and_then(|p| {
                let parent = p.parent()?;
                let gitsmith = parent.join("gitsmith");
                if gitsmith.exists() {
                    Some(gitsmith)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                // Fallback to cargo run
                std::path::PathBuf::from("cargo")
            });

        let output = if gitsmith_path.to_string_lossy() == "cargo" {
            Command::new("cargo")
                .args([
                    "run",
                    "--manifest-path",
                    "/home/master/p/gitsmith/Cargo.toml",
                    "--bin",
                    "gitsmith",
                    "--",
                ])
                .args(args)
                .env("HOME", &self.home_dir)
                .env("GITSMITH_PASSWORD", "test")
                .stdout(Stdio::piped()) // Capture stdout for JSON
                .stderr(Stdio::inherit()) // Still show stderr
                .output()
                .await?
        } else {
            Command::new(gitsmith_path)
                .args(args)
                .env("HOME", &self.home_dir)
                .env("GITSMITH_PASSWORD", "test")
                .stdout(Stdio::piped()) // Capture stdout for JSON
                .stderr(Stdio::inherit()) // Still show stderr
                .output()
                .await?
        };

        let result = CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::new(), // stderr is inherited, not captured
            success: output.status.success(),
            _exit_code: output.status.code().unwrap_or(-1),
        };

        // Also print the JSON output for visibility
        if !result.stdout.is_empty() {
            println!("{}", result.stdout);
        }

        Ok(result)
    }

    /// Run a gitsmith command expecting success
    pub async fn run_success(&self, args: &[&str]) -> Result<CommandOutput> {
        // Check if this is a command that outputs JSON
        let is_json = args.contains(&"--json");

        let output = if is_json {
            self.run_json(args).await?
        } else {
            self.run(args).await?
        };

        if !output.success {
            anyhow::bail!("Command failed: gitsmith {}", args.join(" "));
        }
        Ok(output)
    }

    /// Run a gitsmith command expecting failure
    pub async fn run_failure(&self, args: &[&str]) -> Result<CommandOutput> {
        let output = self.run(args).await?;
        if output.success {
            anyhow::bail!(
                "Command unexpectedly succeeded: gitsmith {}",
                args.join(" ")
            );
        }
        Ok(output)
    }

    /// Run command with custom environment variables
    pub async fn run_with_env(
        &self,
        args: &[&str],
        env: Vec<(&str, &str)>,
    ) -> Result<CommandOutput> {
        println!("    $ gitsmith {}", args.join(" "));
        for (key, val) in &env {
            println!("      env: {}={}", key, val);
        }

        // Determine the path to the gitsmith binary
        let gitsmith_path = std::env::current_exe()
            .ok()
            .and_then(|p| {
                let parent = p.parent()?;
                let gitsmith = parent.join("gitsmith");
                if gitsmith.exists() {
                    Some(gitsmith)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                // Fallback to cargo run
                std::path::PathBuf::from("cargo")
            });

        let mut cmd = if gitsmith_path.to_string_lossy() == "cargo" {
            let mut cmd = Command::new("cargo");
            cmd.args([
                "run",
                "--manifest-path",
                "/home/master/p/gitsmith/Cargo.toml",
                "--bin",
                "gitsmith",
                "--",
            ]);
            cmd.args(args);
            cmd
        } else {
            let mut cmd = Command::new(gitsmith_path);
            cmd.args(args);
            cmd
        };

        cmd.env("HOME", &self.home_dir)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        for (key, val) in env {
            cmd.env(key, val);
        }

        let output = cmd.output().await?;

        // Capture stdout only (stderr goes directly to terminal)
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        // Print stdout for visibility
        if !stdout.is_empty() {
            print!("{}", stdout);
        }

        let result = CommandOutput {
            stdout,
            stderr: String::new(),
            success: output.status.success(),
            _exit_code: output.status.code().unwrap_or(-1),
        };

        Ok(result)
    }
}

/// Command output structure
#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
    pub _exit_code: i32,
}

impl CommandOutput {
    /// Check if stdout contains a string
    #[allow(dead_code)]
    pub fn stdout_contains(&self, text: &str) -> bool {
        self.stdout.contains(text)
    }

    /// Check if stderr contains a string
    #[allow(dead_code)]
    pub fn stderr_contains(&self, text: &str) -> bool {
        self.stderr.contains(text)
    }

    /// Parse stdout as JSON
    #[allow(dead_code)]
    pub fn stdout_json<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_str(&self.stdout)
            .with_context(|| format!("Failed to parse JSON from stdout: {}", self.stdout))
    }

    /// Parse stdout as a list of pull requests
    pub fn parse_pr_list(&self) -> Result<Vec<crate::helpers::PullRequest>> {
        self.stdout_json()
    }
}
