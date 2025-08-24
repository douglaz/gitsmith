use anyhow::Result;
use clap::Parser;
use colored::*;
use tracing::info;

mod cli;
mod helpers;
mod relay;
mod tests;

use cli::Cli;
use relay::RelayManager;
use tests::{account, pull_request, repository, sync};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing subscriber - output to stderr
    let filter = if cli.verbose {
        "gitsmith_integration_tests=debug,gitsmith=debug"
    } else {
        "gitsmith_integration_tests=info,gitsmith=info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();

    info!("Starting GitSmith integration tests");

    // Start relay if needed and build initial relay list
    let (_relay_manager, base_relay_list) = if !cli.skip_relay_setup {
        println!("{}", "🔌 Setting up test relay...".cyan());
        info!("Starting local relay manager");
        let manager = RelayManager::start(cli.verbose).await?;
        let url = manager.get_url();
        info!("Relay ready at {}", url);
        println!("  ✓ Relay started at {}", url.green());
        (Some(manager), vec![url])
    } else {
        info!("Skipping relay setup (--skip-relay-setup flag)");
        (None, Vec::new())
    };

    let verbose = cli.verbose;

    match cli.command {
        cli::Commands::All {
            keep_temp,
            mut relays,
        } => {
            // Build final relay list
            let mut relay_list = base_relay_list;
            relay_list.append(&mut relays);

            // Ensure we have at least one relay
            if relay_list.is_empty() {
                anyhow::bail!(
                    "No relay URLs available. Either start the local relay or provide --relay URLs"
                );
            }

            run_all_tests(verbose, keep_temp, &relay_list).await
        }
        cli::Commands::Account {
            keep_temp,
            mut relays,
        } => {
            let mut relay_list = base_relay_list;
            relay_list.append(&mut relays);
            if relay_list.is_empty() {
                anyhow::bail!(
                    "No relay URLs available. Either start the local relay or provide --relay URLs"
                );
            }
            run_account_tests(verbose, keep_temp, &relay_list).await
        }
        cli::Commands::Repo {
            keep_temp,
            mut relays,
        } => {
            let mut relay_list = base_relay_list;
            relay_list.append(&mut relays);
            if relay_list.is_empty() {
                anyhow::bail!(
                    "No relay URLs available. Either start the local relay or provide --relay URLs"
                );
            }
            run_repository_tests(verbose, keep_temp, &relay_list).await
        }
        cli::Commands::Pr {
            keep_temp,
            mut relays,
        } => {
            let mut relay_list = base_relay_list;
            relay_list.append(&mut relays);
            if relay_list.is_empty() {
                anyhow::bail!(
                    "No relay URLs available. Either start the local relay or provide --relay URLs"
                );
            }
            run_pr_tests(verbose, keep_temp, &relay_list).await
        }
        cli::Commands::Sync {
            keep_temp,
            mut relays,
        } => {
            let mut relay_list = base_relay_list;
            relay_list.append(&mut relays);
            if relay_list.is_empty() {
                anyhow::bail!(
                    "No relay URLs available. Either start the local relay or provide --relay URLs"
                );
            }
            run_sync_tests(verbose, keep_temp, &relay_list).await
        }
    }
}

async fn run_all_tests(verbose: bool, keep_temp: bool, relays: &[String]) -> Result<()> {
    println!("{}", "🧪 Running all GitSmith integration tests...".bold());
    println!();

    let mut total_tests = 0;
    let mut failed_tests = 0;

    // Account tests
    println!("{}", "📝 Account Management Tests".blue().bold());
    let (passed, failed) = account::run_tests(verbose, keep_temp, relays).await?;
    total_tests += passed + failed;
    failed_tests += failed;

    // Repository tests
    println!();
    println!("{}", "📁 Repository Initialization Tests".blue().bold());
    let (passed, failed) = repository::run_tests(verbose, keep_temp, relays).await?;
    total_tests += passed + failed;
    failed_tests += failed;

    // PR tests
    println!();
    println!("{}", "🔀 Pull Request Workflow Tests".blue().bold());
    let (passed, failed) = pull_request::run_tests(verbose, keep_temp, relays).await?;
    total_tests += passed + failed;
    failed_tests += failed;

    // Sync tests
    println!();
    println!("{}", "🔄 List and Sync Tests".blue().bold());
    let (passed, failed) = sync::run_tests(verbose, keep_temp, relays).await?;
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

async fn run_account_tests(verbose: bool, keep_temp: bool, relays: &[String]) -> Result<()> {
    println!("{}", "📝 Running Account Management Tests".blue().bold());
    let (passed, failed) = account::run_tests(verbose, keep_temp, relays).await?;
    print_test_summary(passed, failed);
    Ok(())
}

async fn run_repository_tests(verbose: bool, keep_temp: bool, relays: &[String]) -> Result<()> {
    println!(
        "{}",
        "📁 Running Repository Initialization Tests".blue().bold()
    );
    let (passed, failed) = repository::run_tests(verbose, keep_temp, relays).await?;
    print_test_summary(passed, failed);
    Ok(())
}

async fn run_pr_tests(verbose: bool, keep_temp: bool, relays: &[String]) -> Result<()> {
    println!("{}", "🔀 Running Pull Request Workflow Tests".blue().bold());
    let (passed, failed) = pull_request::run_tests(verbose, keep_temp, relays).await?;
    print_test_summary(passed, failed);
    Ok(())
}

async fn run_sync_tests(verbose: bool, keep_temp: bool, relays: &[String]) -> Result<()> {
    println!("{}", "🔄 Running List and Sync Tests".blue().bold());
    let (passed, failed) = sync::run_tests(verbose, keep_temp, relays).await?;
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
