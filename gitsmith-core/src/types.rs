use nostr::EventId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Repository configuration for Nostr announcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoAnnouncement {
    pub identifier: String,
    pub name: String,
    pub description: String,
    pub clone_urls: Vec<String>,
    pub relays: Vec<String>,
    pub web: Vec<String>,
    pub root_commit: String,
    pub maintainers: Vec<String>, // npubs
    pub grasp_servers: Vec<String>,
}

/// Result of publishing to Nostr
#[derive(Debug, Serialize)]
pub struct PublishResult {
    pub event_id: EventId,
    pub nostr_url: String,
    pub successes: Vec<String>,
    pub failures: Vec<(String, String)>, // (relay, error)
}

/// Git state for Kind 30618 events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitState {
    pub identifier: String,
    pub refs: HashMap<String, String>, // ref_name -> commit_hash
}

/// Configuration for publishing
#[derive(Debug, Clone)]
pub struct PublishConfig {
    pub timeout_secs: u64,
    pub wait_for_send: bool,
}
