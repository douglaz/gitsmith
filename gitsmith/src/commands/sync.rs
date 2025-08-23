use anyhow::{Context, Result};
use clap::Args;
use gitsmith_core::{detect_from_git, get_git_state};
use nostr_sdk::{Alphabet, Client, Filter, Kind, RelayPoolNotification, SingleLetterTag};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Args)]
pub struct SyncArgs {
    /// Repository path
    #[arg(long, default_value = ".")]
    pub repo_path: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn handle_sync_command(args: SyncArgs) -> Result<()> {
    // Get repository info
    let repo_announcement = detect_from_git(&args.repo_path)
        .context("Failed to detect repository. Make sure you're in a git repository")?;

    // Get local git state
    let local_state = get_git_state(&args.repo_path, &repo_announcement.identifier)?;

    println!("Repository: {name}", name = repo_announcement.name);
    println!(
        "Identifier: {identifier}",
        identifier = repo_announcement.identifier
    );
    println!();

    // Display local state
    println!("Local Git State:");
    println!("{:-<40}", "");
    for (ref_name, commit) in &local_state.refs {
        println!(
            "{ref_name:<20} {commit}",
            commit = &commit[..8.min(commit.len())]
        );
    }
    println!();

    // If relays are configured, fetch remote state
    if !repo_announcement.relays.is_empty() {
        println!(
            "Fetching state from {count} relay(s)...",
            count = repo_announcement.relays.len()
        );

        let client = Client::default();

        for relay_url in &repo_announcement.relays {
            client.add_relay(relay_url).await?;
        }

        client.connect().await;

        // Wait for connections to establish
        gitsmith_core::ensure_relay_connected(5)
            .await
            .context("Failed to connect to relays")?;

        // Create filter for state events (Kind 30618)
        let filter = Filter::new().kind(Kind::Custom(30618)).custom_tag(
            SingleLetterTag::lowercase(Alphabet::D),
            &repo_announcement.identifier,
        );

        // Subscribe to events
        client.subscribe(filter, None).await?;

        // Collect events for a few seconds
        let mut state_events = Vec::new();
        let timeout = tokio::time::sleep(Duration::from_secs(3));
        tokio::pin!(timeout);

        let mut notifications = client.notifications();

        loop {
            tokio::select! {
                _ = &mut timeout => break,
                notification = notifications.recv() => {
                    if let Ok(notification) = notification
                        && let RelayPoolNotification::Event { event, .. } = notification
                            && event.kind == Kind::Custom(30618) {
                                state_events.push(*event);
                            }
                }
            }
        }

        if !state_events.is_empty() {
            println!("\nRemote Nostr State:");
            println!("{:-<40}", "");

            // Get the most recent state event
            state_events.sort_by_key(|e| std::cmp::Reverse(e.created_at));

            if let Some(latest_state) = state_events.first() {
                // Parse refs from content (JSON)
                if let Ok(refs) = serde_json::from_str::<serde_json::Value>(&latest_state.content)
                    && let Some(refs_obj) = refs.as_object()
                {
                    for (ref_name, commit) in refs_obj {
                        if let Some(commit_str) = commit.as_str() {
                            println!(
                                "{ref_name:<20} {commit}",
                                commit = &commit_str[..8.min(commit_str.len())]
                            );
                        }
                    }
                }

                println!(
                    "\nLast updated: {timestamp}",
                    timestamp = chrono::DateTime::from_timestamp(
                        latest_state.created_at.as_u64() as i64,
                        0
                    )
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "Unknown".to_string())
                );
            }
        } else {
            println!("\nNo remote state found on Nostr relays");
        }
    } else {
        println!("No relays configured. Run 'gitsmith init' to configure relays.");
    }

    Ok(())
}
