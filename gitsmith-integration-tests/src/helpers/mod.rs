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

/// Retry listing PRs with exponential backoff
pub async fn list_prs_with_retry(
    runner: &GitsmithRunner,
    repo_path: &str,
    max_retries: u32,
) -> Result<Vec<PullRequest>> {
    let delays = [1, 2, 4]; // Fixed delays: 1s, 2s, 4s

    for attempt in 0..=max_retries.min(delays.len() as u32) {
        let output = runner.run_success(&["list", "--repo-path", repo_path, "--json"])?;

        let prs = output.parse_pr_list()?;

        if !prs.is_empty() || attempt == max_retries {
            return Ok(prs);
        }

        if attempt < max_retries {
            let delay = delays.get(attempt as usize).unwrap_or(&4);
            eprintln!(
                "      No PRs found, retrying in {} second(s)... (attempt {}/{})",
                delay,
                attempt + 1,
                max_retries
            );
            sleep(Duration::from_secs(*delay)).await;
        }
    }

    Ok(vec![])
}
