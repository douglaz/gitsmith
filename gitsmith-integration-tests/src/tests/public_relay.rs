use anyhow::Result;
use colored::*;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info};

use crate::helpers::{GitsmithRunner, TestContext};

/// Wait for events to appear on a public relay with extended timeout and exponential backoff
async fn wait_for_pr_with_backoff(
    runner: &GitsmithRunner,
    repo_path: &str,
    expected_count: usize,
    max_wait_minutes: u64,
) -> Result<Vec<Value>> {
    let start = std::time::Instant::now();
    let max_duration = Duration::from_secs(max_wait_minutes * 60);

    // Start with 1 second, double each retry up to 30 seconds
    let mut retry_delay = Duration::from_secs(1);
    let max_retry_delay = Duration::from_secs(30);
    let mut attempt = 0;

    loop {
        attempt += 1;

        // Try to list PRs
        let output = runner
            .run_success(&["list", "--repo-path", repo_path, "--json"])
            .await?;

        if let Ok(prs) = serde_json::from_str::<Vec<Value>>(&output.stdout) {
            if prs.len() >= expected_count {
                let elapsed = start.elapsed();
                info!(
                    "Found {} PRs after {} attempts ({:.1}s)",
                    prs.len(),
                    attempt,
                    elapsed.as_secs_f64()
                );
                return Ok(prs);
            }

            debug!(
                "Attempt {}: Found {} PRs, expecting {}",
                attempt,
                prs.len(),
                expected_count
            );
        }

        // Check if we've exceeded max wait time
        if start.elapsed() >= max_duration {
            let elapsed = start.elapsed();
            anyhow::bail!(
                "Timeout: PRs did not appear after {} attempts over {:.1} seconds (max: {} minutes)",
                attempt,
                elapsed.as_secs_f64(),
                max_wait_minutes
            );
        }

        // Calculate time remaining
        let remaining = max_duration - start.elapsed();
        let next_delay = retry_delay.min(remaining);

        // Log progress periodically
        if attempt % 10 == 0 || retry_delay >= Duration::from_secs(10) {
            let elapsed = start.elapsed();
            println!(
                "  ⏳ Still waiting for PRs... (attempt {}, {:.1}s elapsed, {:.1}s remaining)",
                attempt,
                elapsed.as_secs_f64(),
                remaining.as_secs_f64()
            );
        }

        // Wait before next retry
        sleep(next_delay).await;

        // Exponential backoff: double the delay, up to max
        retry_delay = (retry_delay * 2).min(max_retry_delay);
    }
}

/// Test PR creation and retrieval with extended timeout for public relays
pub async fn test_public_relay_pr_eventual_consistency(
    keep_temp: bool,
    relays: &[String],
    max_wait_minutes: u64,
) -> Result<()> {
    println!(
        "  🌐 Testing PR eventual consistency on public relay (max wait: {} minutes)",
        max_wait_minutes
    );

    let ctx = TestContext::new("public-relay-test", keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir);

    {
        println!("  📂 Test directory: {}", ctx.home_dir.display());
    }

    // Create test repository with commits
    ctx.setup_git_repo(5)?;
    let repo_path = ctx.repo_path.to_str().unwrap();

    // Generate unique identifiers to avoid conflicts
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let nsec = format!("{:064x}", timestamp % 1000000);
    let identifier = format!("pub-relay-test-{}", timestamp);

    // Login
    runner
        .run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])
        .await?;

    // Initialize repository
    let relay_args: Vec<String> = relays
        .iter()
        .flat_map(|r| vec!["--relay".to_string(), r.clone()])
        .collect();

    let mut init_args = vec![
        "init",
        "--identifier",
        &identifier,
        "--name",
        "Public Relay Test",
        "--description",
        "Testing eventual consistency",
        "--nsec",
        &nsec,
        "--repo-path",
        repo_path,
    ];

    let relay_args_refs: Vec<&str> = relay_args.iter().map(|s| s.as_str()).collect();
    init_args.extend(relay_args_refs.iter());

    runner.run_success(&init_args).await?;

    println!("  📤 Sending PR to public relay...");

    // Send a PR
    let pr_output = runner
        .run_success(&[
            "send",
            "--title",
            "Test PR for Public Relay",
            "--description",
            "This PR tests eventual consistency on public relays",
            "--repo-path",
            repo_path,
            "--password",
            "test",
            "HEAD~2",
        ])
        .await?;

    {
        println!("  Send output: {}", pr_output.stderr);
    }

    // Verify PR was sent successfully
    if !pr_output.stderr.contains("Pull request sent successfully") {
        anyhow::bail!("PR was not sent successfully: {}", pr_output.stderr);
    }

    println!("  ⏱️  Waiting for PR to appear on relay (this may take several minutes)...");

    // Wait for PR to appear with extended timeout and backoff
    let prs = wait_for_pr_with_backoff(&runner, repo_path, 1, max_wait_minutes).await?;

    // Verify PR content
    let pr = &prs[0];
    let title = pr["title"].as_str().unwrap_or("");
    if title != "Test PR for Public Relay" {
        anyhow::bail!(
            "PR title mismatch. Expected 'Test PR for Public Relay', got '{}'",
            title
        );
    }

    println!("  ✅ PR successfully appeared on public relay!");

    // Test multiple PRs if we have time
    if max_wait_minutes >= 5 {
        println!("  📤 Sending additional PRs to test multiple events...");

        // Send 2 more PRs
        for i in 3..=4 {
            let title = format!("Additional PR {}", i - 2);
            runner
                .run_success(&[
                    "send",
                    "--title",
                    &title,
                    "--description",
                    "Testing multiple PRs",
                    "--repo-path",
                    repo_path,
                    "--password",
                    "test",
                    &format!("HEAD~{}", i),
                ])
                .await?;
        }

        println!("  ⏱️  Waiting for all 3 PRs to appear...");

        // Wait for all 3 PRs
        let all_prs = wait_for_pr_with_backoff(&runner, repo_path, 3, max_wait_minutes).await?;

        println!(
            "  ✅ All {} PRs successfully appeared on public relay!",
            all_prs.len()
        );
    }

    Ok(())
}

/// Run public relay tests with configurable timeout
pub async fn run_tests(
    keep_temp: bool,
    relays: &[String],
    max_wait_minutes: u64,
) -> Result<(usize, usize)> {
    let mut passed = 0;
    let mut failed = 0;

    // Only run if we have relays configured
    if relays.is_empty() {
        println!("  ⚠️  No relays provided, skipping public relay tests");
        return Ok((0, 0));
    }

    // Test eventual consistency
    match test_public_relay_pr_eventual_consistency(keep_temp, relays, max_wait_minutes).await {
        Ok(_) => {
            println!(
                "  {} test_public_relay_pr_eventual_consistency",
                "✓".green()
            );
            passed += 1;
        }
        Err(e) => {
            println!(
                "  {} test_public_relay_pr_eventual_consistency: {}",
                "✗".red(),
                e.to_string().red()
            );
            failed += 1;
        }
    }

    Ok((passed, failed))
}
