pub mod account;
pub mod events;
pub mod patches;
pub mod pull_request;
pub mod repo;
pub mod types;

use anyhow::Result;
use std::time::Duration;

// Re-export main types and functions for convenience
pub use events::{
    KIND_GIT_PATCH, KIND_GIT_REPO_ANNOUNCEMENT, KIND_GIT_STATE, build_announcement_event,
    build_state_event,
};
pub use repo::{announce_repository, detect_from_git, get_git_state, update_git_config};
pub use types::{GitState, PublishConfig, PublishResult, RepoAnnouncement};

/// Wait for relay connections to be established with timeout
///
/// This function provides a smart delay to allow relay connections to establish.
/// It waits a minimum of 500ms (typical connection time) up to `timeout_secs`.
///
/// Returns Ok(()) when ready to proceed with relay operations.
pub async fn ensure_relay_connected(timeout_secs: u64) -> Result<()> {
    let start = tokio::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    // The nostr-sdk Client manages connections internally and handles retries.
    // Since we can't directly check connection status in the current API,
    // we use a smart delay strategy:
    // - Wait a minimum of 500ms for fast connections
    // - But return early if we hit that minimum
    // - This is much better than always waiting 2 seconds

    while start.elapsed() < timeout {
        // Small delay to allow connections to establish
        tokio::time::sleep(Duration::from_millis(100)).await;

        // After 500ms, most connections that will succeed have done so
        if start.elapsed() >= Duration::from_millis(500) {
            // The client will handle any connection issues internally
            // when we try to send/receive events
            break;
        }
    }

    // At this point, either:
    // 1. We've waited at least 500ms (typical connection time)
    // 2. We've hit the timeout (connections probably won't succeed)
    // The client will handle retries and queueing internally

    // In the future, when nostr-sdk exposes connection status,
    // we can check client.pool().relays() or similar

    Ok(())
}
