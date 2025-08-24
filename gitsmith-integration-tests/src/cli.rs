use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gitsmith-integration-tests")]
#[command(about = "Integration test suite for gitsmith")]
#[command(version)]
pub struct Cli {
    /// Skip automatic relay setup (use existing relay)
    #[arg(long, global = true)]
    pub skip_relay_setup: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run all integration tests
    All {
        /// Keep temporary directories after tests
        #[arg(long)]
        keep_temp: bool,

        /// Additional relay URLs to use for tests (can be specified multiple times)
        #[arg(long = "relay")]
        relays: Vec<String>,
    },

    /// Run account management tests
    Account {
        /// Keep temporary directories after tests
        #[arg(long)]
        keep_temp: bool,

        /// Additional relay URLs to use for tests (can be specified multiple times)
        #[arg(long = "relay")]
        relays: Vec<String>,
    },

    /// Run repository initialization tests
    Repo {
        /// Keep temporary directories after tests
        #[arg(long)]
        keep_temp: bool,

        /// Additional relay URLs to use for tests (can be specified multiple times)
        #[arg(long = "relay")]
        relays: Vec<String>,
    },

    /// Run pull request workflow tests
    Pr {
        /// Keep temporary directories after tests
        #[arg(long)]
        keep_temp: bool,

        /// Additional relay URLs to use for tests (can be specified multiple times)
        #[arg(long = "relay")]
        relays: Vec<String>,
    },

    /// Run list and sync tests
    Sync {
        /// Keep temporary directories after tests
        #[arg(long)]
        keep_temp: bool,

        /// Additional relay URLs to use for tests (can be specified multiple times)
        #[arg(long = "relay")]
        relays: Vec<String>,
    },

    /// Run public relay tests with extended timeouts
    PublicRelay {
        /// Keep temporary directories after tests
        #[arg(long)]
        keep_temp: bool,

        /// Relay URLs to test (required for public relay tests)
        #[arg(long = "relay", required = true)]
        relays: Vec<String>,

        /// Maximum time to wait for events to appear (in minutes)
        #[arg(long, default_value = "5")]
        max_wait_minutes: u64,
    },
}
