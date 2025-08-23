use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gitsmith-integration-tests")]
#[command(about = "Integration test suite for GitSmith")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run all integration tests
    All {
        /// Show verbose output
        #[arg(long, short = 'v')]
        verbose: bool,
        
        /// Keep temporary directories after tests
        #[arg(long)]
        keep_temp: bool,
    },
    
    /// Run account management tests
    Account {
        /// Show verbose output
        #[arg(long, short = 'v')]
        verbose: bool,
        
        /// Keep temporary directories after tests
        #[arg(long)]
        keep_temp: bool,
    },
    
    /// Run repository initialization tests
    Repo {
        /// Show verbose output
        #[arg(long, short = 'v')]
        verbose: bool,
        
        /// Keep temporary directories after tests
        #[arg(long)]
        keep_temp: bool,
    },
    
    /// Run pull request workflow tests
    Pr {
        /// Show verbose output
        #[arg(long, short = 'v')]
        verbose: bool,
        
        /// Keep temporary directories after tests
        #[arg(long)]
        keep_temp: bool,
    },
    
    /// Run list and sync tests
    Sync {
        /// Show verbose output
        #[arg(long, short = 'v')]
        verbose: bool,
        
        /// Keep temporary directories after tests
        #[arg(long)]
        keep_temp: bool,
    },
}