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
use tests::{account, public_relay, pull_request, repository, sync};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();

    info!("Starting gitsmith integration tests");

    // Start relays if needed and build initial relay list
    let managers = if !cli.skip_relay_setup {
        println!("{}", "🔌 Setting up test relays...".cyan());
        info!("Starting local relay managers");
        let relay_managers = RelayManager::start_multiple().await?;
        for manager in &relay_managers {
            let url = manager.get_url();
            info!("Relay ready at {}", url);
            println!("  ✓ Relay started at {}", url.green());
        }
        relay_managers
    } else {
        info!("Skipping relay setup (--skip-relay-setup flag)");
        Vec::new()
    };
    let mut relay_list = Vec::new();

    for manager in &managers {
        let url = manager.get_url();
        relay_list.push(url);
    }

    match cli.command {
        cli::Commands::All {
            keep_temp,
            mut relays,
        } => {
            // Build final relay list
            relay_list.append(&mut relays);

            // Ensure we have at least one relay
            if relay_list.is_empty() {
                anyhow::bail!(
                    "No relay URLs available. Either start the local relay or provide --relay URLs"
                );
            }

            run_all_tests(keep_temp, &relay_list).await
        }
        cli::Commands::Account {
            keep_temp,
            mut relays,
        } => {
            relay_list.append(&mut relays);
            if relay_list.is_empty() {
                anyhow::bail!(
                    "No relay URLs available. Either start the local relay or provide --relay URLs"
                );
            }
            run_account_tests(keep_temp).await
        }
        cli::Commands::Repo {
            keep_temp,
            mut relays,
        } => {
            relay_list.append(&mut relays);
            if relay_list.is_empty() {
                anyhow::bail!(
                    "No relay URLs available. Either start the local relay or provide --relay URLs"
                );
            }
            run_repository_tests(keep_temp, &relay_list).await
        }
        cli::Commands::Pr {
            keep_temp,
            mut relays,
        } => {
            relay_list.append(&mut relays);
            if relay_list.is_empty() {
                anyhow::bail!(
                    "No relay URLs available. Either start the local relay or provide --relay URLs"
                );
            }
            run_pr_tests(keep_temp, &relay_list).await
        }
        cli::Commands::Sync {
            keep_temp,
            mut relays,
        } => {
            relay_list.append(&mut relays);
            if relay_list.is_empty() {
                anyhow::bail!(
                    "No relay URLs available. Either start the local relay or provide --relay URLs"
                );
            }
            run_sync_tests(keep_temp, &relay_list).await
        }
        cli::Commands::PublicRelay {
            keep_temp,
            relays,
            max_wait_minutes,
        } => {
            // Public relay tests don't use local relay
            if relays.is_empty() {
                anyhow::bail!("Public relay tests require at least one --relay URL");
            }
            run_public_relay_tests(keep_temp, &relays, max_wait_minutes).await
        }
    }
}

async fn run_all_tests(keep_temp: bool, relays: &[String]) -> Result<()> {
    println!("{}", "🧪 Running all gitsmith integration tests...".bold());
    println!();

    let mut total_tests = 0;
    let mut failed_tests = 0;

    // Account tests
    println!("{}", "📝 Account Management Tests".blue().bold());
    let (passed, failed) = account::run_tests(keep_temp).await?;
    total_tests += passed + failed;
    failed_tests += failed;

    // Repository tests
    println!();
    println!("{}", "📁 Repository Initialization Tests".blue().bold());
    let (passed, failed) = repository::run_tests(keep_temp, relays).await?;
    total_tests += passed + failed;
    failed_tests += failed;

    // PR tests
    println!();
    println!("{}", "🔀 Pull Request Workflow Tests".blue().bold());
    let (passed, failed) = pull_request::run_tests(keep_temp, relays).await?;
    total_tests += passed + failed;
    failed_tests += failed;

    // Sync tests
    println!();
    println!("{}", "🔄 List and Sync Tests".blue().bold());
    let (passed, failed) = sync::run_tests(keep_temp, relays).await?;
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

async fn run_account_tests(keep_temp: bool) -> Result<()> {
    println!("{}", "📝 Running Account Management Tests".blue().bold());
    let (passed, failed) = account::run_tests(keep_temp).await?;
    print_test_summary(passed, failed);
    Ok(())
}

async fn run_repository_tests(keep_temp: bool, relays: &[String]) -> Result<()> {
    println!(
        "{}",
        "📁 Running Repository Initialization Tests".blue().bold()
    );
    let (passed, failed) = repository::run_tests(keep_temp, relays).await?;
    print_test_summary(passed, failed);
    Ok(())
}

async fn run_pr_tests(keep_temp: bool, relays: &[String]) -> Result<()> {
    println!("{}", "🔀 Running Pull Request Workflow Tests".blue().bold());
    let (passed, failed) = pull_request::run_tests(keep_temp, relays).await?;
    print_test_summary(passed, failed);
    Ok(())
}

async fn run_sync_tests(keep_temp: bool, relays: &[String]) -> Result<()> {
    println!("{}", "🔄 Running List and Sync Tests".blue().bold());
    let (passed, failed) = sync::run_tests(keep_temp, relays).await?;
    print_test_summary(passed, failed);
    Ok(())
}

async fn run_public_relay_tests(
    keep_temp: bool,
    relays: &[String],
    max_wait_minutes: u64,
) -> Result<()> {
    println!("{}", "🌐 Running Public Relay Tests".blue().bold());
    println!("  Maximum wait time: {} minutes", max_wait_minutes);
    println!("  Testing with relay(s): {}", relays.join(", "));
    println!();

    let (passed, failed) = public_relay::run_tests(keep_temp, relays, max_wait_minutes).await?;
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
