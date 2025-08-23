use crate::helpers::{GitsmithRunner, TestContext, assert_contains, assert_file_contains};
use anyhow::Result;
use colored::*;
use std::process::Command;

/// Run all repository initialization tests
pub async fn run_tests(verbose: bool, keep_temp: bool) -> Result<(usize, usize)> {
    let mut passed = 0;
    let mut failed = 0;

    // Test initializing a new repository
    match test_init_new_repo(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_init_new_repo", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_init_new_repo: {}", "✗".red(), e);
            failed += 1;
        }
    }

    // Test initializing an existing repository
    match test_init_existing_repo(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_init_existing_repo", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_init_existing_repo: {}", "✗".red(), e);
            failed += 1;
        }
    }

    // Test config persistence
    match test_init_config_persistence(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_init_config_persistence", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_init_config_persistence: {}", "✗".red(), e);
            failed += 1;
        }
    }

    // Test detect from git
    match test_detect_from_git(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_detect_from_git", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_detect_from_git: {}", "✗".red(), e);
            failed += 1;
        }
    }

    Ok((passed, failed))
}

async fn test_init_new_repo(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_init_new_repo", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    // Setup git repo
    ctx.setup_git_repo(3)?;

    // Generate test key
    let nsec = TestContext::generate_test_key();

    // Initialize repository
    let output = runner.run_success(&[
        "init",
        "--identifier",
        "test-repo",
        "--name",
        "Test Repository",
        "--description",
        "A test repository",
        "--relay",
        "wss://relay.damus.io",
        "--relay",
        "wss://nos.lol",
        "--nsec",
        &nsec,
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
        "--output",
        "json",
    ])?;

    // Verify JSON output
    assert_contains(&output.stdout, "\"event_id\"", "Should output event ID")?;
    assert_contains(&output.stdout, "\"nostr_url\"", "Should output nostr URL")?;
    assert_contains(
        &output.stdout,
        "\"successes\"",
        "Should show successful relays",
    )?;

    Ok(())
}

async fn test_init_existing_repo(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_init_existing_repo", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    // Setup existing git repo with remote
    ctx.setup_git_repo(5)?;

    // Add a remote origin
    Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/test/repo.git",
        ])
        .current_dir(&ctx.repo_path)
        .output()?;

    let nsec = TestContext::generate_test_key();

    // Initialize with minimal args (should detect from git)
    let output = runner.run_success(&[
        "init",
        "--identifier",
        "existing-repo",
        "--name",
        "Existing Repo",
        "--description",
        "Testing with existing repo",
        "--relay",
        "wss://relay.nostr.band",
        "--nsec",
        &nsec,
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
        "--output",
        "minimal",
    ])?;

    // Should output just the nostr URL
    assert_contains(&output.stdout, "nostr://", "Should output nostr URL")?;

    Ok(())
}

async fn test_init_config_persistence(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_init_config_persistence", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    ctx.setup_git_repo(2)?;

    let nsec = TestContext::generate_test_key();
    let identifier = format!("config-test-{}", uuid::Uuid::new_v4());

    // Initialize repository
    runner.run_success(&[
        "init",
        "--identifier",
        &identifier,
        "--name",
        "Config Test",
        "--description",
        "Testing configuration persistence",
        "--relay",
        "wss://relay.damus.io",
        "--relay",
        "wss://nos.lol",
        "--nsec",
        &nsec,
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
    ])?;

    // Check git config was updated
    let git_config_path = ctx.repo_path.join(".git/config");
    assert_file_contains(&git_config_path, "[nostr]")?;
    assert_file_contains(&git_config_path, &format!("identifier = {}", identifier))?;
    assert_file_contains(&git_config_path, "name = Config Test")?;
    assert_file_contains(&git_config_path, "relay = wss://relay.damus.io")?;
    assert_file_contains(&git_config_path, "relay = wss://nos.lol")?;

    Ok(())
}

async fn test_detect_from_git(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_detect_from_git", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    ctx.setup_git_repo(2)?;

    // First initialize a repo
    let nsec = TestContext::generate_test_key();
    let identifier = format!("detect-test-{}", uuid::Uuid::new_v4());

    runner.run_success(&[
        "init",
        "--identifier",
        &identifier,
        "--name",
        "Detect Test",
        "--description",
        "Testing detection",
        "--relay",
        "wss://relay.example.com",
        "--nsec",
        &nsec,
        "--repo-path",
        &ctx.repo_path.to_string_lossy(),
    ])?;

    // Generate announcement from existing repo (should detect saved config)
    let output =
        runner.run_success(&["generate", "--repo-path", &ctx.repo_path.to_string_lossy()])?;

    // Verify it detected the saved configuration
    assert_contains(
        &output.stdout,
        &identifier,
        "Should detect saved identifier",
    )?;
    assert_contains(&output.stdout, "Detect Test", "Should detect saved name")?;
    assert_contains(
        &output.stdout,
        "Testing detection",
        "Should detect saved description",
    )?;
    assert_contains(
        &output.stdout,
        "wss://relay.example.com",
        "Should detect saved relay",
    )?;

    Ok(())
}
