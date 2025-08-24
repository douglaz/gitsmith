pub mod assertions;
pub mod runner;
pub mod setup;
pub mod types;

pub use assertions::*;
pub use runner::*;
pub use setup::*;
pub use types::*;

use anyhow::Result;
use tokio::time::{Duration, sleep};
use tracing::{debug, info, warn};

/// Retry listing PRs with patient exponential backoff suitable for public relays
pub async fn list_prs_with_retry(
    runner: &GitsmithRunner,
    repo_path: &str,
    max_retries: u32,
) -> Result<Vec<PullRequest>> {
    info!(
        "Starting PR list retry for repo_path: {} with max_retries: {}",
        repo_path, max_retries
    );

    // Patient retry strategy with exponential backoff
    // Suitable for both local and public relays
    let mut delay_secs = 1u64;
    let max_delay_secs = 30u64;

    for attempt in 0..=max_retries {
        debug!("List attempt {}/{}", attempt + 1, max_retries + 1);
        let output = runner
            .run_success(&["list", "--repo-path", repo_path, "--json"])
            .await?;

        let prs = output.parse_pr_list()?;
        debug!("Found {} PRs on attempt {}", prs.len(), attempt + 1);

        if !prs.is_empty() || attempt == max_retries {
            if !prs.is_empty() {
                info!(
                    "Successfully found {} PRs after {} attempt(s)",
                    prs.len(),
                    attempt + 1
                );
            } else {
                warn!("No PRs found after {} attempts", max_retries + 1);
            }
            return Ok(prs);
        }

        if attempt < max_retries {
            info!(
                "No PRs found yet, will retry in {} seconds (attempt {}/{})",
                delay_secs,
                attempt + 1,
                max_retries
            );
            eprintln!(
                "      No PRs found, retrying in {} second(s)... (attempt {}/{})",
                delay_secs,
                attempt + 1,
                max_retries
            );
            sleep(Duration::from_secs(delay_secs)).await;

            // Exponential backoff: double the delay, up to max
            let old_delay = delay_secs;
            delay_secs = (delay_secs * 2).min(max_delay_secs);
            debug!(
                "Exponential backoff: {} -> {} seconds",
                old_delay, delay_secs
            );
        }
    }

    Ok(vec![])
}
