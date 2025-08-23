use anyhow::{Context, Result};
use clap::Args;
use gitsmith_core::{detect_from_git, pull_request};
use std::path::PathBuf;

#[derive(Args)]
pub struct ListArgs {
    /// Repository path
    #[arg(long, default_value = ".")]
    pub repo_path: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn handle_list_command(args: ListArgs) -> Result<()> {
    // Get repository info
    let repo_announcement = detect_from_git(&args.repo_path)
        .context("Failed to detect repository. Make sure you're in a git repository")?;

    if repo_announcement.relays.is_empty() {
        println!("Warning: No relays configured for repository");
        println!("Please run 'gitsmith init' first to configure relays");
        return Ok(());
    }

    // For now, we'll use a placeholder public key
    // In a real implementation, this would come from the repository's maintainer info
    let repo_coordinate = format!("30617:placeholder:{}", repo_announcement.identifier);

    println!(
        "Fetching pull requests from {} relay(s)...",
        repo_announcement.relays.len()
    );

    // List pull requests
    let prs = pull_request::list_pull_requests(&repo_coordinate, repo_announcement.relays.clone())
        .await?;

    if args.json {
        // Output as JSON
        let json = serde_json::to_string_pretty(&prs)?;
        println!("{}", json);
    } else {
        // Human-readable output
        if prs.is_empty() {
            println!("No pull requests found");
        } else {
            println!("\nFound {} pull request(s):\n", prs.len());
            println!("{:-<80}", "");

            for (i, pr) in prs.iter().enumerate() {
                println!("PR #{}", i + 1);
                println!("{}", pull_request::format_pull_request(pr));
                println!("{:-<80}", "");
            }
        }
    }

    Ok(())
}
