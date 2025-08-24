use anyhow::Result;
use nostr::{Event, EventId, Filter};
use nostr_sdk::{Client, RelayPoolNotification};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use strum::{Display, EnumString};

use crate::patches::{KIND_PULL_REQUEST, KIND_PULL_REQUEST_UPDATE};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum PullRequestStatus {
    Open,
    Updated,
}

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
    pub status: PullRequestStatus,
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

    // Collect events with early exit when found
    let mut events = Vec::new();
    let timeout = tokio::time::sleep(Duration::from_millis(1500)); // Wait up to 1.5 seconds
    tokio::pin!(timeout);

    let mut notifications = client.notifications();
    let mut last_event_time = tokio::time::Instant::now();

    loop {
        tokio::select! {
            _ = &mut timeout => break,
            notification = notifications.recv() => {
                if let Ok(notification) = notification {
                    if let RelayPoolNotification::Event { event, .. } = notification
                        && (event.kind == KIND_PULL_REQUEST || event.kind == KIND_PULL_REQUEST_UPDATE) {
                            events.push(*event);
                            last_event_time = tokio::time::Instant::now();
                        }
                    // If we've found events and 100ms have passed without new ones, exit early
                    if !events.is_empty() && last_event_time.elapsed() > Duration::from_millis(100) {
                        break;
                    }
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
                    existing.status = PullRequestStatus::Updated;
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
        PullRequestStatus::Updated
    } else {
        PullRequestStatus::Open
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

    output.push_str(&format!("Title: {title}\n", title = pr.title));
    output.push_str(&format!(
        "Author: {author}...\n",
        author = &pr.author[0..16]
    ));
    output.push_str(&format!("Status: {status}\n", status = pr.status));
    output.push_str(&format!(
        "Patches: {patches_count}\n",
        patches_count = pr.patches_count
    ));

    if let Some(commit) = &pr.root_commit {
        output.push_str(&format!(
            "Root: {commit}...\n",
            commit = &commit[0..8.min(commit.len())]
        ));
    }

    if !pr.description.is_empty() {
        output.push_str(&format!("\n{description}\n", description = pr.description));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_pull_request_status_serialization() {
        // Test Open status
        let open_status = PullRequestStatus::Open;
        let json = serde_json::to_string(&open_status).unwrap();
        assert_eq!(json, r#""open""#);

        // Test Updated status
        let updated_status = PullRequestStatus::Updated;
        let json = serde_json::to_string(&updated_status).unwrap();
        assert_eq!(json, r#""updated""#);
    }

    #[test]
    fn test_pull_request_status_deserialization() {
        // Test deserializing "open"
        let open_status: PullRequestStatus = serde_json::from_str(r#""open""#).unwrap();
        assert_eq!(open_status, PullRequestStatus::Open);

        // Test deserializing "updated"
        let updated_status: PullRequestStatus = serde_json::from_str(r#""updated""#).unwrap();
        assert_eq!(updated_status, PullRequestStatus::Updated);
    }

    #[test]
    fn test_pull_request_status_display() {
        assert_eq!(PullRequestStatus::Open.to_string(), "open");
        assert_eq!(PullRequestStatus::Updated.to_string(), "updated");
    }

    #[test]
    fn test_pull_request_serialization() {
        let pr = PullRequest {
            id: "test-id-123".to_string(),
            title: "Test PR".to_string(),
            description: "This is a test pull request".to_string(),
            author: "npub1234567890abcdef".to_string(),
            created_at: 1234567890,
            updated_at: Some(1234567900),
            patches_count: 3,
            root_commit: Some("abc123def456".to_string()),
            status: PullRequestStatus::Open,
        };

        let json = serde_json::to_value(&pr).unwrap();

        // Verify the structure
        assert_eq!(json["id"], "test-id-123");
        assert_eq!(json["title"], "Test PR");
        assert_eq!(json["description"], "This is a test pull request");
        assert_eq!(json["author"], "npub1234567890abcdef");
        assert_eq!(json["created_at"], 1234567890);
        assert_eq!(json["updated_at"], 1234567900);
        assert_eq!(json["patches_count"], 3);
        assert_eq!(json["root_commit"], "abc123def456");
        assert_eq!(json["status"], "open");
    }

    #[test]
    fn test_pull_request_deserialization() {
        let json = r#"{
            "id": "test-id-456",
            "title": "Another PR",
            "description": "Description here",
            "author": "npub9876543210fedcba",
            "created_at": 9876543210,
            "updated_at": null,
            "patches_count": 1,
            "root_commit": null,
            "status": "updated"
        }"#;

        let pr: PullRequest = serde_json::from_str(json).unwrap();

        assert_eq!(pr.id, "test-id-456");
        assert_eq!(pr.title, "Another PR");
        assert_eq!(pr.description, "Description here");
        assert_eq!(pr.author, "npub9876543210fedcba");
        assert_eq!(pr.created_at, 9876543210);
        assert_eq!(pr.updated_at, None);
        assert_eq!(pr.patches_count, 1);
        assert_eq!(pr.root_commit, None);
        assert_eq!(pr.status, PullRequestStatus::Updated);
    }

    #[test]
    fn test_format_pull_request() {
        let pr = PullRequest {
            id: "id123".to_string(),
            title: "Test Title".to_string(),
            description: "Test description".to_string(),
            author: "npub1234567890123456789".to_string(),
            created_at: 1000000,
            updated_at: None,
            patches_count: 2,
            root_commit: Some("commit12345678".to_string()),
            status: PullRequestStatus::Open,
        };

        let formatted = format_pull_request(&pr);

        assert!(formatted.contains("Title: Test Title"));
        assert!(formatted.contains("Author: npub123456789012"));
        assert!(formatted.contains("Status: open"));
        assert!(formatted.contains("Patches: 2"));
        assert!(formatted.contains("Root: commit12"));
        assert!(formatted.contains("Test description"));
    }
}
