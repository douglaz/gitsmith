use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;
use uuid::Uuid;

/// Test context that holds temporary directories and configuration
pub struct TestContext {
    pub temp_dir: TempDir,
    pub repo_path: PathBuf,
    pub home_dir: PathBuf,
    pub verbose: bool,
    pub keep_temp: bool,
    pub test_name: String,
}

impl TestContext {
    /// Create a new test context with temporary directories
    pub fn new(test_name: &str, verbose: bool, keep_temp: bool) -> Result<Self> {
        let temp_dir = if keep_temp {
            TempDir::new_in("/tmp")?
        } else {
            TempDir::new()?
        };

        let repo_path = temp_dir.path().join("test-repo");
        let home_dir = temp_dir.path().join("home");

        // Create directories
        std::fs::create_dir_all(&repo_path)?;
        std::fs::create_dir_all(&home_dir)?;

        if verbose {
            println!("  📂 Test directory: {}", temp_dir.path().display());
        }

        Ok(Self {
            temp_dir,
            repo_path,
            home_dir,
            verbose,
            keep_temp,
            test_name: test_name.to_string(),
        })
    }

    /// Get the path to the gitsmith-accounts.json file
    pub fn accounts_file(&self) -> PathBuf {
        self.home_dir
            .join(".config/gitsmith/gitsmith-accounts.json")
    }

    /// Setup a git repository with initial commits
    pub fn setup_git_repo(&self, num_commits: usize) -> Result<()> {
        // Initialize repo
        Command::new("git")
            .arg("init")
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to initialize git repo")?;

        // Set user config
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&self.repo_path)
            .output()?;

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&self.repo_path)
            .output()?;

        // Create commits
        for i in 0..num_commits {
            let filename = format!("file{}.txt", i);
            let filepath = self.repo_path.join(&filename);
            std::fs::write(&filepath, format!("Content {}", i))?;

            Command::new("git")
                .args(["add", &filename])
                .current_dir(&self.repo_path)
                .output()?;

            Command::new("git")
                .args(["commit", "-m", &format!("Commit {}", i)])
                .current_dir(&self.repo_path)
                .output()?;
        }

        if self.verbose {
            println!("    ✓ Created git repo with {} commits", num_commits);
        }

        Ok(())
    }

    /// Generate a test private key (32 bytes hex)
    pub fn generate_test_key() -> String {
        // Generate two UUIDs to get enough random bytes
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        // Convert to hex and ensure we have exactly 64 characters
        let hex1 = format!("{:032x}", uuid1.as_u128());
        let hex2 = format!("{:032x}", uuid2.as_u128());

        // Take first 32 chars from hex1 and first 32 from hex2
        format!("{}{}", &hex1[..32], &hex2[..32])
    }

    /// Create a test account and login
    pub fn setup_test_account(&self, password: &str) -> Result<String> {
        let nsec = Self::generate_test_key();

        // Ensure config directory exists
        let config_dir = self.home_dir.join(".config/gitsmith");
        std::fs::create_dir_all(&config_dir)?;

        if self.verbose {
            println!("    ✓ Created test account with key");
        }

        Ok(nsec)
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        if self.keep_temp {
            println!(
                "  📌 Keeping test directory: {}",
                self.temp_dir.path().display()
            );
        }
    }
}
