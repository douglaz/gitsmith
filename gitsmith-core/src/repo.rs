use anyhow::{Context, Result, bail};
use git2::Repository;
use nostr::{Keys, RelayUrl, ToBech32};
use nostr_sdk::Client;
use std::path::Path;
use std::time::Duration;

use crate::events;
use crate::types::*;

/// Publish repository announcement to Nostr (Kind 30617)
pub async fn announce_repository(
    announcement: RepoAnnouncement,
    private_key_hex: &str,
    config: PublishConfig,
) -> Result<PublishResult> {
    // Parse keys
    let keys = Keys::parse(private_key_hex)?;

    // Build announcement event
    let event = events::build_announcement_event(&announcement, &keys)?;
    let event_id = event.id;

    // Create client
    let client = Client::new(keys.clone());

    // Add relays
    for relay_url in &announcement.relays {
        let url = RelayUrl::parse(relay_url)?;
        client.add_relay(url).await?;
    }

    // Connect to relays
    client.connect().await;

    // Wait for connections to establish
    crate::ensure_relay_connected(5)
        .await
        .context("Failed to connect to relays")?;

    // Send event
    client.send_event(&event).await?;

    // Wait a bit for propagation if requested
    if config.wait_for_send {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    // Build nostr URL (using naddr format)
    let npub = keys.public_key().to_bech32()?;
    let first_relay = announcement
        .relays
        .first()
        .map(|r| format!("/{}", r.replace("wss://", "").replace("ws://", "")))
        .unwrap_or_default();

    let nostr_url = format!(
        "nostr://{}{}/{}",
        npub, first_relay, announcement.identifier
    );

    Ok(PublishResult {
        event_id,
        nostr_url,
        successes: announcement.relays.clone(),
        failures: vec![],
    })
}

/// Detect repository information from git
pub fn detect_from_git(repo_path: &Path) -> Result<RepoAnnouncement> {
    let repo = Repository::open(repo_path)
        .with_context(|| format!("Failed to open git repository at {repo_path:?}"))?;

    // Get repo name from directory
    let name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unnamed")
        .to_string();

    // Get root commit
    let root_commit = get_root_commit(&repo)?;

    // Get origin URL if exists
    let clone_urls = if let Ok(remote) = repo.find_remote("origin") {
        if let Some(url) = remote.url() {
            vec![url.to_string()]
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    Ok(RepoAnnouncement {
        identifier: sanitize_identifier(&name),
        name,
        description: String::new(),
        clone_urls,
        relays: vec![],
        web: vec![],
        root_commit,
        maintainers: vec![],
        grasp_servers: vec![],
    })
}

/// Get the root commit of a repository
fn get_root_commit(repo: &Repository) -> Result<String> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME | git2::Sort::REVERSE)?;

    let mut root = None;
    for oid in revwalk {
        root = Some(oid?);
    }

    if let Some(oid) = root {
        Ok(oid.to_string())
    } else {
        bail!("No commits found in repository")
    }
}

/// Get current git state (refs and HEAD)
pub fn get_git_state(repo_path: &Path, identifier: &str) -> Result<GitState> {
    let repo = Repository::open(repo_path)?;
    let mut refs = std::collections::HashMap::new();

    // Get all references
    for reference in repo.references()? {
        let reference = reference?;
        if let Some(name) = reference.name()
            && let Some(target) = reference.target()
        {
            refs.insert(name.to_string(), target.to_string());
        }
    }

    // Get HEAD
    if let Ok(head) = repo.head()
        && let Some(target) = head.target()
    {
        refs.insert("HEAD".to_string(), target.to_string());
    }

    Ok(GitState {
        identifier: identifier.to_string(),
        refs,
    })
}

/// Sanitize identifier to be valid for Nostr
fn sanitize_identifier(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// Update git config with nostr remote
pub fn update_git_config(repo_path: &Path, nostr_url: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    let mut config = repo.config()?;

    // Save the nostr URL in git config
    config.set_str("nostr.url", nostr_url)?;

    Ok(())
}
