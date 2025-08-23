use anyhow::Result;
use clap::Parser;
use colored::*;

mod cli;
mod helpers;
mod tests;

use cli::Cli;
use tests::{account, pull_request, repository, sync};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        cli::Commands::All { verbose, keep_temp } => {
            run_all_tests(verbose, keep_temp).await
        }
        cli::Commands::Account { verbose, keep_temp } => {
            run_account_tests(verbose, keep_temp).await
        }
        cli::Commands::Repo { verbose, keep_temp } => {
            run_repository_tests(verbose, keep_temp).await
        }
        cli::Commands::Pr { verbose, keep_temp } => {
            run_pr_tests(verbose, keep_temp).await
        }
        cli::Commands::Sync { verbose, keep_temp } => {
            run_sync_tests(verbose, keep_temp).await
        }
    }
}

async fn run_all_tests(verbose: bool, keep_temp: bool) -> Result<()> {
    println!("{}", "🧪 Running all GitSmith integration tests...".bold());
    println!();
    
    let mut total_tests = 0;
    let mut failed_tests = 0;
    
    // Account tests
    println!("{}", "📝 Account Management Tests".blue().bold());
    let (passed, failed) = account::run_tests(verbose, keep_temp).await?;
    total_tests += passed + failed;
    failed_tests += failed;
    
    // Repository tests
    println!();
    println!("{}", "📁 Repository Initialization Tests".blue().bold());
    let (passed, failed) = repository::run_tests(verbose, keep_temp).await?;
    total_tests += passed + failed;
    failed_tests += failed;
    
    // PR tests
    println!();
    println!("{}", "🔀 Pull Request Workflow Tests".blue().bold());
    let (passed, failed) = pull_request::run_tests(verbose, keep_temp).await?;
    total_tests += passed + failed;
    failed_tests += failed;
    
    // Sync tests
    println!();
    println!("{}", "🔄 List and Sync Tests".blue().bold());
    let (passed, failed) = sync::run_tests(verbose, keep_temp).await?;
    total_tests += passed + failed;
    failed_tests += failed;
    
    // Summary
    println!();
    println!("{}", "═".repeat(60).blue());
    if failed_tests == 0 {
        println!(
            "{} {} tests passed!",
            "✅".green(),
            format!("All {}", total_tests).green().bold()
        );
    } else {
        println!(
            "{} {} tests passed, {} failed",
            "❌".red(),
            (total_tests - failed_tests).to_string().green(),
            failed_tests.to_string().red().bold()
        );
        std::process::exit(1);
    }
    
    Ok(())
}

async fn run_account_tests(verbose: bool, keep_temp: bool) -> Result<()> {
    println!("{}", "📝 Running Account Management Tests".blue().bold());
    let (passed, failed) = account::run_tests(verbose, keep_temp).await?;
    print_test_summary(passed, failed);
    Ok(())
}

async fn run_repository_tests(verbose: bool, keep_temp: bool) -> Result<()> {
    println!("{}", "📁 Running Repository Initialization Tests".blue().bold());
    let (passed, failed) = repository::run_tests(verbose, keep_temp).await?;
    print_test_summary(passed, failed);
    Ok(())
}

async fn run_pr_tests(verbose: bool, keep_temp: bool) -> Result<()> {
    println!("{}", "🔀 Running Pull Request Workflow Tests".blue().bold());
    let (passed, failed) = pull_request::run_tests(verbose, keep_temp).await?;
    print_test_summary(passed, failed);
    Ok(())
}

async fn run_sync_tests(verbose: bool, keep_temp: bool) -> Result<()> {
    println!("{}", "🔄 Running List and Sync Tests".blue().bold());
    let (passed, failed) = sync::run_tests(verbose, keep_temp).await?;
    print_test_summary(passed, failed);
    Ok(())
}

fn print_test_summary(passed: usize, failed: usize) {
    println!();
    if failed == 0 {
        println!(
            "{} {} tests passed!",
            "✅".green(),
            format!("{}", passed).green().bold()
        );
    } else {
        println!(
            "{} {} tests passed, {} failed",
            "❌".red(),
            passed.to_string().green(),
            failed.to_string().red().bold()
        );
        std::process::exit(1);
    }
}