use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gitsmith-integration-tests")]
#[command(about = "Integration test suite for GitSmith")]
#[command(version)]
pub struct Cli {
    /// Skip automatic relay setup (use existing relay)
    #[arg(long, global = true)]
    pub skip_relay_setup: bool,

    /// Show verbose output
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,

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
}
