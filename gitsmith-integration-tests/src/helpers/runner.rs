use anyhow::{Context, Result};
use assert_cmd::Command;
use std::path::Path;

/// Runner for gitsmith commands
pub struct GitsmithRunner {
    verbose: bool,
    home_dir: String,
}

impl GitsmithRunner {
    /// Create a new gitsmith runner with HOME environment set
    pub fn new(home_dir: &Path, verbose: bool) -> Self {
        Self {
            verbose,
            home_dir: home_dir.to_string_lossy().to_string(),
        }
    }

    /// Run a gitsmith command with arguments
    pub fn run(&self, args: &[&str]) -> Result<CommandOutput> {
        if self.verbose {
            println!("    $ gitsmith {}", args.join(" "));
        }

        let mut cmd = Command::cargo_bin("gitsmith").unwrap_or_else(|_| {
            // Fallback to using the built binary directly
            let mut cmd = Command::new("cargo");
            cmd.args([
                "run",
                "--manifest-path",
                "/home/master/p/gitsmith/Cargo.toml",
                "--bin",
                "gitsmith",
                "--",
            ]);
            cmd
        });

        cmd.args(args)
            .env("HOME", &self.home_dir)
            .env("GITSMITH_PASSWORD", "test"); // Default test password

        let output = cmd.output()?;

        let result = CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            success: output.status.success(),
            _exit_code: output.status.code().unwrap_or(-1),
        };

        if self.verbose {
            if !result.stderr.is_empty() {
                println!("      stderr: {}", result.stderr.trim());
            }
            if !result.stdout.is_empty() {
                println!("      stdout: {}", result.stdout.trim());
            }
        }

        Ok(result)
    }

    /// Run a gitsmith command expecting success
    pub fn run_success(&self, args: &[&str]) -> Result<CommandOutput> {
        let output = self.run(args)?;
        if !output.success {
            anyhow::bail!(
                "Command failed: gitsmith {}\nstderr: {}\nstdout: {}",
                args.join(" "),
                output.stderr,
                output.stdout
            );
        }
        Ok(output)
    }

    /// Run a gitsmith command expecting failure
    pub fn run_failure(&self, args: &[&str]) -> Result<CommandOutput> {
        let output = self.run(args)?;
        if output.success {
            anyhow::bail!(
                "Command unexpectedly succeeded: gitsmith {}\nstdout: {}",
                args.join(" "),
                output.stdout
            );
        }
        Ok(output)
    }

    /// Run command with custom environment variables
    pub fn run_with_env(&self, args: &[&str], env: Vec<(&str, &str)>) -> Result<CommandOutput> {
        if self.verbose {
            println!("    $ gitsmith {}", args.join(" "));
            for (key, val) in &env {
                println!("      env: {}={}", key, val);
            }
        }

        let mut cmd = Command::cargo_bin("gitsmith").unwrap_or_else(|_| {
            let mut cmd = Command::new("cargo");
            cmd.args([
                "run",
                "--manifest-path",
                "/home/master/p/gitsmith/Cargo.toml",
                "--bin",
                "gitsmith",
                "--",
            ]);
            cmd
        });

        cmd.args(args).env("HOME", &self.home_dir);

        for (key, val) in env {
            cmd.env(key, val);
        }

        let output = cmd.output()?;

        let result = CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            success: output.status.success(),
            _exit_code: output.status.code().unwrap_or(-1),
        };

        if self.verbose && !result.stderr.is_empty() {
            println!("      stderr: {}", result.stderr.trim());
        }

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
