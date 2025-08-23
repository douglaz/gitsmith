use crate::helpers::{GitsmithRunner, TestContext, assert_contains};
use anyhow::Result;
use colored::*;

/// Run all pull request workflow tests
pub async fn run_tests(verbose: bool, keep_temp: bool) -> Result<(usize, usize)> {
    let mut passed = 0;
    let mut failed = 0;

    // Test sending a simple PR
    match test_send_pr_simple(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_send_pr_simple", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_send_pr_simple: {}", "✗".red(), e);
            failed += 1;
        }
    }

    // Test sending PR with title and description
    match test_send_pr_with_title_description(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_send_pr_with_title_description", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_send_pr_with_title_description: {}", "✗".red(), e);
            failed += 1;
        }
    }

    // Test sending PR with no commits
    match test_send_pr_no_commits(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_send_pr_no_commits", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_send_pr_no_commits: {}", "✗".red(), e);
            failed += 1;
        }
    }

    // Test sending PR with multiple patches
    match test_send_pr_multiple_patches(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_send_pr_multiple_patches", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_send_pr_multiple_patches: {}", "✗".red(), e);
            failed += 1;
        }
    }

    Ok((passed, failed))
}

async fn test_send_pr_simple(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_send_pr_simple", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    // Setup repo with commits
    ctx.setup_git_repo(3)?;

    // Setup account
    let nsec = TestContext::generate_test_key();
    runner.run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])?;

    // Initialize repo
    runner.run_success(&[
        "init",
        "--identifier",
        "pr-test",
        "--name",
        "PR Test Repo",
        "--description",
        "Testing PRs",
        "--relay",
        "wss://relay.damus.io",
        "--nsec",
        &nsec,
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
    ])?;

    // Send PR
    let output = runner.run_success(&[
        "send",
        "--title",
        "Test PR",
        "--description",
        "This is a test PR",
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
        "--password",
        "test",
        "HEAD~1",
    ])?;

    assert_contains(
        &output.stderr,
        "Generated 1 patch(es)",
        "Should generate patches",
    )?;
    assert_contains(&output.stderr, "Created", "Should create events")?;
    assert_contains(
        &output.stderr,
        "✅ Pull request sent successfully",
        "Should send successfully",
    )?;

    Ok(())
}

async fn test_send_pr_with_title_description(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_send_pr_title_desc", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    ctx.setup_git_repo(5)?;

    let nsec = TestContext::generate_test_key();
    runner.run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])?;

    runner.run_success(&[
        "init",
        "--identifier",
        "pr-title-test",
        "--name",
        "PR Title Test",
        "--description",
        "Testing with title/desc",
        "--relay",
        "wss://relay.nostr.band",
        "--nsec",
        &nsec,
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
    ])?;

    // Send PR with specific title and description
    let output = runner.run_success(&[
        "send",
        "--title",
        "Feature: Add new functionality",
        "--description",
        "This PR adds important new features:\n- Feature A\n- Feature B",
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
        "--password",
        "test",
        "HEAD~2",
    ])?;

    assert_contains(
        &output.stderr,
        "Generated 2 patch(es)",
        "Should generate 2 patches",
    )?;
    assert_contains(&output.stderr, "✅", "Should succeed")?;

    Ok(())
}

async fn test_send_pr_no_commits(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_send_pr_no_commits", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    // Setup repo with only 1 commit
    ctx.setup_git_repo(1)?;

    let nsec = TestContext::generate_test_key();
    runner.run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])?;

    runner.run_success(&[
        "init",
        "--identifier",
        "pr-no-commits",
        "--name",
        "No Commits Test",
        "--description",
        "Testing with no commits to send",
        "--relay",
        "wss://relay.damus.io",
        "--nsec",
        &nsec,
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
    ])?;

    // Try to send PR from HEAD~1 (should fail as there's only 1 commit)
    let output = runner.run_failure(&[
        "send",
        "--title",
        "Empty PR",
        "--description",
        "Should fail",
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
        "--password",
        "test",
        "HEAD~1",
    ])?;

    assert_contains(
        &output.stderr,
        "Not enough commits",
        "Should fail with not enough commits",
    )?;

    Ok(())
}

async fn test_send_pr_multiple_patches(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_send_pr_multiple", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    // Create repo with many commits
    ctx.setup_git_repo(10)?;

    let nsec = TestContext::generate_test_key();
    runner.run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])?;

    runner.run_success(&[
        "init",
        "--identifier",
        "pr-multiple",
        "--name",
        "Multiple Patches Test",
        "--description",
        "Testing with multiple patches",
        "--relay",
        "wss://relay.damus.io",
        "--nsec",
        &nsec,
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
    ])?;

    // Send PR with 5 patches
    let output = runner.run_success(&[
        "send",
        "--title",
        "Multiple commits PR",
        "--description",
        "This PR contains multiple patches",
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
        "--password",
        "test",
        "HEAD~5",
    ])?;

    assert_contains(
        &output.stderr,
        "Generated 5 patch(es)",
        "Should generate 5 patches",
    )?;
    assert_contains(&output.stderr, "Created", "Should create events")?;
    assert_contains(&output.stderr, "✅", "Should succeed")?;

    Ok(())
}
