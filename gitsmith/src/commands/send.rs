use anyhow::Result;
use clap::Args;
use gitsmith_core::{account, patches};
use nostr_sdk::Client;
use rpassword::read_password;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Args)]
pub struct SendArgs {
    /// Commits to send (e.g., HEAD~2)
    #[arg(default_value = "HEAD~1")]
    pub since: String,

    /// Title for the pull request
    #[arg(long, short = 't')]
    pub title: Option<String>,

    /// Description for the pull request
    #[arg(long, short = 'd')]
    pub description: Option<String>,

    /// Reply to an existing PR (event ID)
    #[arg(long)]
    pub in_reply_to: Option<String>,

    /// Repository path
    #[arg(long, default_value = ".")]
    pub repo_path: PathBuf,
}

pub async fn handle_send_command(args: SendArgs) -> Result<()> {
    // Get account keys
    eprint!("Enter password: ");
    io::stderr().flush()?;
    let password = read_password()?;
    let keys = account::get_active_keys(&password)?;

    // Get repository info
    let repo_announcement = gitsmith_core::detect_from_git(&args.repo_path)?;

    // Generate patches
    eprintln!("Generating patches from {since}...", since = args.since);
    let patches = patches::generate_patches(&args.repo_path, Some(&args.since), None)?;

    if patches.is_empty() {
        eprintln!("No patches to send");
        return Ok(());
    }

    eprintln!("Generated {count} patch(es)", count = patches.len());

    // Get title and description
    let title = if let Some(t) = args.title {
        t
    } else {
        eprint!("Enter PR title: ");
        io::stderr().flush()?;
        let mut title = String::new();
        io::stdin().read_line(&mut title)?;
        title.trim().to_string()
    };

    let description = if let Some(d) = args.description {
        d
    } else {
        eprint!("Enter PR description (optional): ");
        io::stderr().flush()?;
        let mut desc = String::new();
        io::stdin().read_line(&mut desc)?;
        desc.trim().to_string()
    };

    // Create repository coordinate
    let repo_coordinate = format!(
        "30617:{pubkey}:{identifier}",
        pubkey = keys.public_key(),
        identifier = repo_announcement.identifier
    );

    // Create PR events
    let events = patches::create_pull_request_event(
        &keys,
        &repo_coordinate,
        &title,
        &description,
        patches,
        &repo_announcement.root_commit,
        args.in_reply_to,
    )?;

    eprintln!("Created {count} events", count = events.len());

    // Send to relays
    if repo_announcement.relays.is_empty() {
        eprintln!("Warning: No relays configured for repository");
        eprintln!("Please run 'gitsmith init' first to configure relays");
        return Ok(());
    }

    let client = Client::new(keys.clone());

    for relay_url in &repo_announcement.relays {
        client.add_relay(relay_url).await?;
    }

    client.connect().await;

    eprintln!(
        "Sending PR to {count} relay(s)...",
        count = repo_announcement.relays.len()
    );

    for event in events {
        client.send_event(&event).await?;
    }

    eprintln!("✅ Pull request sent successfully!");

    Ok(())
}
