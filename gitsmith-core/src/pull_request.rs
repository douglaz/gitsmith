use anyhow::Result;
use nostr::{Event, EventId, Filter};
use nostr_sdk::{Client, RelayPoolNotification};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::patches::{KIND_PULL_REQUEST, KIND_PULL_REQUEST_UPDATE};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub created_at: u64,
    pub updated_at: Option<u64>,
    pub patches_count: usize,
    pub root_commit: Option<String>,
    pub status: String,
}

/// List pull requests for a repository
pub async fn list_pull_requests(
    repo_coordinate: &str,
    relays: Vec<String>,
) -> Result<Vec<PullRequest>> {
    let client = Client::default();

    // Add relays
    for relay_url in &relays {
        client.add_relay(relay_url).await?;
    }

    // Connect to relays
    client.connect().await;

    // Wait a bit for connections
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Create filter for PR events
    let mut filter = Filter::new();
    filter = filter.kinds(vec![KIND_PULL_REQUEST, KIND_PULL_REQUEST_UPDATE]);
    // Add custom tag for repository coordinate
    filter = filter.custom_tag(
        nostr::SingleLetterTag::lowercase(nostr::Alphabet::A),
        repo_coordinate,
    );

    // Subscribe to events
    client.subscribe(filter, None).await?;

    // Collect events for a few seconds
    let mut events = Vec::new();
    let timeout = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(timeout);

    let mut notifications = client.notifications();

    loop {
        tokio::select! {
            _ = &mut timeout => break,
            notification = notifications.recv() => {
                if let Ok(notification) = notification
                    && let RelayPoolNotification::Event { event, .. } = notification
                        && (event.kind == KIND_PULL_REQUEST || event.kind == KIND_PULL_REQUEST_UPDATE) {
                            events.push(*event);
                        }
            }
        }
    }

    // Process events into pull requests
    let mut prs: HashMap<EventId, PullRequest> = HashMap::new();

    for event in events {
        let pr = event_to_pull_request(&event)?;

        // Check if this is an update to existing PR
        let is_update = event.kind == KIND_PULL_REQUEST_UPDATE;

        if is_update {
            // Find the original PR this updates
            if let Some(original_id) = find_reply_to(&event)
                && let Some(existing) = prs.get_mut(&original_id)
            {
                // Update the existing PR
                if existing.created_at < event.created_at.as_u64() {
                    existing.updated_at = Some(event.created_at.as_u64());
                    existing.description = pr.description;
                    existing.status = "updated".to_string();
                }
            }
        } else {
            // New PR
            prs.insert(event.id, pr);
        }
    }

    // Convert to vector and sort by creation time
    let mut result: Vec<PullRequest> = prs.into_values().collect();
    result.sort_by_key(|pr| std::cmp::Reverse(pr.created_at));

    Ok(result)
}

/// Convert an event to a PullRequest
fn event_to_pull_request(event: &Event) -> Result<PullRequest> {
    let title = get_tag_value(event, "subject").unwrap_or_else(|| "Untitled PR".to_string());

    let root_commit = get_tag_value(event, "c");

    // Count patch references
    let patches_count = event
        .tags
        .iter()
        .filter(|tag| {
            tag.as_slice().len() > 1
                && tag.as_slice()[0] == "e"
                && tag.as_slice().get(2).is_some_and(|s| s == "patch")
        })
        .count();

    let status = if event.kind == KIND_PULL_REQUEST_UPDATE {
        "updated".to_string()
    } else {
        "open".to_string()
    };

    Ok(PullRequest {
        id: event.id.to_string(),
        title,
        description: event.content.clone(),
        author: event.pubkey.to_string(),
        created_at: event.created_at.as_u64(),
        updated_at: None,
        patches_count,
        root_commit,
        status,
    })
}

/// Get a tag value from an event
fn get_tag_value(event: &Event, tag_name: &str) -> Option<String> {
    event
        .tags
        .iter()
        .find(|tag| tag.as_slice().len() > 1 && tag.as_slice()[0] == tag_name)
        .and_then(|tag| tag.as_slice().get(1))
        .map(|s| s.to_string())
}

/// Find the event ID this event is replying to
fn find_reply_to(event: &Event) -> Option<EventId> {
    event
        .tags
        .iter()
        .find(|tag| {
            tag.as_slice().len() > 1
                && tag.as_slice()[0] == "e"
                && tag.as_slice().get(3).is_some_and(|s| s == "reply")
        })
        .and_then(|tag| tag.as_slice().get(1))
        .and_then(|s| s.parse().ok())
}

/// Format a pull request for display
pub fn format_pull_request(pr: &PullRequest) -> String {
    let mut output = String::new();

    output.push_str(&format!("Title: {}\n", pr.title));
    output.push_str(&format!("Author: {}...\n", &pr.author[0..16]));
    output.push_str(&format!("Status: {}\n", pr.status));
    output.push_str(&format!("Patches: {}\n", pr.patches_count));

    if let Some(commit) = &pr.root_commit {
        output.push_str(&format!("Root: {}...\n", &commit[0..8.min(commit.len())]));
    }

    if !pr.description.is_empty() {
        output.push_str(&format!("\n{}\n", pr.description));
    }

    output
}
