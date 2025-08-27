use anyhow::Result;
use clap::Args;
use gitsmith_core::{account, patches};
use nostr_sdk::Client;
use rpassword::read_password;
use std::io::{self, Write};
use std::path::PathBuf;
use tracing::{debug, info, warn};

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

    /// Password to decrypt account keys (will prompt if not provided)
    #[arg(long, env = "GITSMITH_PASSWORD")]
    pub password: Option<String>,
}

pub async fn handle_send_command(args: SendArgs) -> Result<()> {
    info!(repository = %args.repo_path.display(), "Starting send command for repository");

    // Get account keys
    debug!("Getting account keys");
    let password = if let Some(pwd) = args.password {
        pwd
    } else {
        eprint!("Enter password: ");
        io::stderr().flush()?;
        read_password()?
    };
    let keys = account::get_active_keys(&password)?;
    info!("Account keys loaded successfully");

    // Get repository info
    debug!(path = %args.repo_path.display(), "Detecting repository info");
    let repo_announcement = gitsmith_core::detect_from_git(&args.repo_path)?;
    info!(name = %repo_announcement.name, identifier = %repo_announcement.identifier, "Repository detected");

    // Generate patches
    eprintln!("Generating patches from {since}...", since = args.since);
    debug!(since = %args.since, "Generating patches from commit range");
    let patches = patches::generate_patches(&args.repo_path, Some(&args.since), None)?;
    info!(count = patches.len(), "Generated patches from commits");

    if patches.is_empty() {
        warn!(since = %args.since, "No patches to send - no commits in range");
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
    debug!(coordinate = %repo_coordinate, "Repository coordinate created");

    // Create PR events
    debug!(title = %title, "Creating PR events");
    let events = patches::create_pull_request_event(
        &keys,
        &repo_coordinate,
        &title,
        &description,
        patches,
        &repo_announcement.root_commit,
        args.in_reply_to,
    )?;

    info!(
        count = events.len(),
        "Created events (patch events + PR event)"
    );
    eprintln!("Created {count} events", count = events.len());

    // Send to relays
    if repo_announcement.relays.is_empty() {
        warn!("No relays configured for repository");
        eprintln!("Warning: No relays configured for repository");
        eprintln!("Please run 'gitsmith init' first to configure relays");
        return Ok(());
    }

    debug!(relays = ?repo_announcement.relays, "Configured relays");

    let client = Client::new(keys.clone());

    for relay_url in &repo_announcement.relays {
        debug!(%relay_url, "Adding relay");
        client.add_relay(relay_url).await?;
    }

    info!(
        count = repo_announcement.relays.len(),
        "Connecting to relays"
    );
    client.connect().await;

    eprintln!(
        "Sending PR to {count} relay(s)...",
        count = repo_announcement.relays.len()
    );

    // Send events with a small delay between them to avoid overwhelming public relays
    // This is especially important for multi-patch PRs which create multiple events
    let mut total_successes = std::collections::HashSet::new();
    let mut total_failures = std::collections::HashMap::new();

    for (i, event) in events.iter().enumerate() {
        if i > 0 {
            debug!(
                delay_ms = 500,
                "Waiting before sending next event to avoid overwhelming relays"
            );
            // Add a 500ms delay between events to give relays time to process
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        debug!(event_num = i + 1, total = events.len(), kind = %event.kind, id = %event.id, "Sending event");

        let output = client.send_event(event).await?;

        // Track successes
        for relay in output.success {
            total_successes.insert(relay.to_string());
        }

        // Track failures
        for (relay, msg) in output.failed {
            total_failures.insert(relay.to_string(), msg);
        }

        info!(
            event_num = i + 1,
            total = events.len(),
            "Event sent successfully"
        );
    }

    // Report results
    let success_count = total_successes.len();
    let failure_count = total_failures.len();

    if success_count > 0 {
        info!(
            relay_count = success_count,
            "All events sent successfully to relays"
        );
        eprintln!("✅ Pull request sent to {} relay(s)!", success_count);
    }

    if failure_count > 0 {
        warn!(failure_count, failures = ?total_failures, "Failed to send to some relays");
        eprintln!("⚠️  Failed to send to {} relay(s)", failure_count);
        for (relay, msg) in &total_failures {
            eprintln!("   - {}: {}", relay, msg);
        }
    }

    if success_count == 0 {
        anyhow::bail!("Failed to send events to any relay");
    }

    Ok(())
}
