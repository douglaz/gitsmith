use anyhow::{Context, Result};
use clap::Args;
use gitsmith_core::{account, detect_from_git, get_repo_owner, pull_request};
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
        eprintln!("Warning: No relays configured for repository");
        eprintln!("Please run 'gitsmith init' first to configure relays");
        return Ok(());
    }

    // Get the repository owner's public key
    // First try to get it from the repo config (set during init)
    // If not found, fall back to active account
    let public_key = if let Some(owner) = get_repo_owner(&args.repo_path)? {
        owner
    } else {
        // Fall back to active account if repo doesn't have owner saved
        account::get_active_public_key().context(
            "Repository owner not found in config and no active account. Please login first with 'gitsmith account login'",
        )?
    };

    let repo_coordinate = format!(
        "30617:{pubkey}:{identifier}",
        pubkey = public_key,
        identifier = repo_announcement.identifier
    );

    eprintln!(
        "Fetching pull requests from {count} relay(s)...",
        count = repo_announcement.relays.len()
    );

    // List pull requests
    let prs = pull_request::list_pull_requests(&repo_coordinate, repo_announcement.relays.clone())
        .await?;

    if args.json {
        // Output as JSON
        let json = serde_json::to_string_pretty(&prs)?;
        println!("{json}");
    } else {
        // Human-readable output
        if prs.is_empty() {
            eprintln!("No pull requests found");
        } else {
            eprintln!("\nFound {count} pull request(s):\n", count = prs.len());
            eprintln!("{:-<80}", "");

            for (i, pr) in prs.iter().enumerate() {
                eprintln!("PR #{num}", num = i + 1);
                eprintln!(
                    "{pr_output}",
                    pr_output = pull_request::format_pull_request(pr)
                );
                eprintln!("{:-<80}", "");
            }
        }
    }

    Ok(())
}
