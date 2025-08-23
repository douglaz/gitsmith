use crate::helpers::{GitsmithRunner, TestContext, assert_contains, assert_pr_exists};
use anyhow::Result;
use colored::*;
use tokio::time::{Duration, sleep};

/// Run all list and sync tests
pub async fn run_tests(verbose: bool, keep_temp: bool) -> Result<(usize, usize)> {
    let mut passed = 0;
    let mut failed = 0;

    // Test listing pull requests
    match test_list_pull_requests(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_list_pull_requests", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_list_pull_requests: {}", "✗".red(), e);
            failed += 1;
        }
    }

    // Test listing empty repo
    match test_list_empty_repo(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_list_empty_repo", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_list_empty_repo: {}", "✗".red(), e);
            failed += 1;
        }
    }

    // Test syncing repository
    match test_sync_repository(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_sync_repository", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_sync_repository: {}", "✗".red(), e);
            failed += 1;
        }
    }

    // Test sync with saved config
    match test_sync_with_saved_config(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_sync_with_saved_config", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_sync_with_saved_config: {}", "✗".red(), e);
            failed += 1;
        }
    }

    // Error handling tests
    match test_invalid_private_key(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_invalid_private_key", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_invalid_private_key: {}", "✗".red(), e);
            failed += 1;
        }
    }

    match test_missing_relays(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_missing_relays", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_missing_relays: {}", "✗".red(), e);
            failed += 1;
        }
    }

    Ok((passed, failed))
}

async fn test_list_pull_requests(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_list_prs", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    ctx.setup_git_repo(5)?;

    let nsec = TestContext::generate_test_key();
    runner.run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])?;

    // Initialize repo
    runner.run_success(&[
        "init",
        "--identifier",
        "list-test",
        "--name",
        "List Test Repo",
        "--description",
        "Testing list functionality",
        "--relay",
        "wss://relay.damus.io",
        "--nsec",
        &nsec,
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
    ])?;

    // Send a PR first
    runner.run_success(&[
        "send",
        "--title",
        "Test PR for listing",
        "--description",
        "This PR will be listed",
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
        "--password",
        "test",
        "HEAD~2",
    ])?;

    // Wait a moment for the PR to propagate to the relay
    sleep(Duration::from_secs(3)).await;

    // List PRs
    let output = runner.run_success(&[
        "list",
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
        "--json",
    ])?;

    // Verify JSON output and parse it
    assert_contains(&output.stdout, "[", "Should output JSON array")?;

    // Parse and verify the PR details
    let prs = output.parse_pr_list()?;

    if prs.is_empty() {
        anyhow::bail!("Expected at least one PR after sending, but list is empty");
    }

    // Find the PR we sent
    let pr = assert_pr_exists(&prs, "Test PR for listing")?;

    // Verify PR details
    if pr.description != "This PR will be listed" {
        anyhow::bail!(
            "PR description mismatch. Expected: 'This PR will be listed', Got: '{}'",
            pr.description
        );
    }

    if pr.patches_count != 2 {
        anyhow::bail!("Expected 2 patches (HEAD~2), got {}", pr.patches_count);
    }

    if verbose {
        println!("    ✓ Found PR with title: {}", pr.title);
        println!("    ✓ PR has {} patches as expected", pr.patches_count);
    }

    Ok(())
}

async fn test_list_empty_repo(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_list_empty", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    ctx.setup_git_repo(2)?;

    let nsec = TestContext::generate_test_key();

    // Initialize repo without sending any PRs
    runner.run_success(&[
        "init",
        "--identifier",
        "empty-list-test",
        "--name",
        "Empty List Test",
        "--description",
        "Testing empty list",
        "--relay",
        "wss://relay.damus.io",
        "--nsec",
        &nsec,
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
    ])?;

    // List PRs (should be empty)
    let output = runner.run_success(&[
        "list",
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
        "--json",
    ])?;

    assert_contains(&output.stdout, "[]", "Should output empty JSON array")?;

    // Also parse to verify it's truly empty
    let prs = output.parse_pr_list()?;
    if !prs.is_empty() {
        anyhow::bail!("Expected empty PR list, but got {} PRs", prs.len());
    }

    if verbose {
        println!("    ✓ Verified PR list is empty");
    }

    Ok(())
}

async fn test_sync_repository(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_sync_repo", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    ctx.setup_git_repo(3)?;

    let nsec = TestContext::generate_test_key();
    runner.run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])?;

    // Initialize repo
    runner.run_success(&[
        "init",
        "--identifier",
        "sync-test",
        "--name",
        "Sync Test Repo",
        "--description",
        "Testing sync",
        "--relay",
        "wss://relay.damus.io",
        "--nsec",
        &nsec,
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
    ])?;

    // Sync repository state (doesn't need password)
    let output = runner.run_success(&["sync", "--repo-path", &ctx.repo_path.to_string_lossy()])?;

    assert_contains(
        &output.stderr,
        "Local Git State:",
        "Should show local git state",
    )?;
    assert_contains(
        &output.stderr,
        "refs/heads/master",
        "Should show master branch",
    )?;

    Ok(())
}

async fn test_sync_with_saved_config(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_sync_saved", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    ctx.setup_git_repo(2)?;

    let nsec = TestContext::generate_test_key();
    runner.run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])?;

    // Initialize repo (saves config)
    runner.run_success(&[
        "init",
        "--identifier",
        "sync-saved-test",
        "--name",
        "Sync Saved Test",
        "--description",
        "Testing sync with saved config",
        "--relay",
        "wss://relay.nostr.band",
        "--nsec",
        &nsec,
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
    ])?;

    // Add a new commit
    let new_file = ctx.repo_path.join("new.txt");
    std::fs::write(&new_file, "new content")?;
    std::process::Command::new("git")
        .args(["add", "new.txt"])
        .current_dir(&ctx.repo_path)
        .output()?;
    std::process::Command::new("git")
        .args(["commit", "-m", "New commit"])
        .current_dir(&ctx.repo_path)
        .output()?;

    // Sync should use saved relay config (doesn't need password)
    let output = runner.run_success(&["sync", "--repo-path", &ctx.repo_path.to_string_lossy()])?;

    assert_contains(
        &output.stderr,
        "Local Git State:",
        "Should sync with saved config",
    )?;
    assert_contains(
        &output.stderr,
        "sync-saved-test",
        "Should use saved identifier",
    )?;

    Ok(())
}

// Error handling tests
async fn test_invalid_private_key(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_invalid_key", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    // Try to login with invalid key
    let output = runner.run_failure(&[
        "account",
        "login",
        "--nsec",
        "invalid-key",
        "--password",
        "test",
    ])?;

    assert_contains(
        &output.stderr,
        "Invalid secret key",
        "Should fail with invalid key error",
    )?;

    Ok(())
}

async fn test_missing_relays(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_missing_relays", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    ctx.setup_git_repo(2)?;

    let nsec = TestContext::generate_test_key();

    // Try to init without relays
    let output = runner.run_failure(&[
        "init",
        "--identifier",
        "no-relays",
        "--name",
        "No Relays",
        "--description",
        "Should fail",
        "--nsec",
        &nsec,
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
    ])?;

    assert_contains(
        &output.stderr,
        "At least one relay is required",
        "Should require relays",
    )?;

    Ok(())
}
