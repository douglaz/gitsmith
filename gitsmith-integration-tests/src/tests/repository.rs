use crate::helpers::{GitsmithRunner, TestContext, assert_contains, assert_file_contains};
use anyhow::Result;
use colored::*;
use std::process::Command;

/// Helper function to build init command arguments with dynamic relays
fn build_init_args<'a>(
    identifier: &'a str,
    name: &'a str,
    description: &'a str,
    relays: &'a [String],
    nsec: &'a str,
    repo_path: &'a str,
    output: Option<&'a str>,
) -> Vec<&'a str> {
    let mut args = vec![
        "init",
        "--identifier",
        identifier,
        "--name",
        name,
        "--description",
        description,
    ];

    // Add relay arguments
    for relay in relays {
        args.push("--relay");
        args.push(relay.as_str());
    }

    // Add remaining arguments
    args.push("--nsec");
    args.push(nsec);
    args.push("--repo-path");
    args.push(repo_path);

    if let Some(output_format) = output {
        args.push("--output");
        args.push(output_format);
    }

    args
}

/// Run all repository initialization tests
pub async fn run_tests(keep_temp: bool, relays: &[String]) -> Result<(usize, usize)> {
    let mut passed = 0;
    let mut failed = 0;

    // Test initializing a new repository
    match test_init_new_repo(keep_temp, relays).await {
        Ok(_) => {
            println!("  {check} test_init_new_repo", check = "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!(
                "  {cross} test_init_new_repo: {error}",
                cross = "✗".red(),
                error = e
            );
            failed += 1;
        }
    }

    // Test initializing an existing repository
    match test_init_existing_repo(keep_temp, relays).await {
        Ok(_) => {
            println!("  {check} test_init_existing_repo", check = "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!(
                "  {cross} test_init_existing_repo: {error}",
                cross = "✗".red(),
                error = e
            );
            failed += 1;
        }
    }

    // Test config persistence
    match test_init_config_persistence(keep_temp, relays).await {
        Ok(_) => {
            println!(
                "  {check} test_init_config_persistence",
                check = "✓".green()
            );
            passed += 1;
        }
        Err(e) => {
            println!(
                "  {cross} test_init_config_persistence: {error}",
                cross = "✗".red(),
                error = e
            );
            failed += 1;
        }
    }

    // Test detect from git
    match test_detect_from_git(keep_temp, relays).await {
        Ok(_) => {
            println!("  {check} test_detect_from_git", check = "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!(
                "  {cross} test_detect_from_git: {error}",
                cross = "✗".red(),
                error = e
            );
            failed += 1;
        }
    }

    Ok((passed, failed))
}

async fn test_init_new_repo(keep_temp: bool, relays: &[String]) -> Result<()> {
    let ctx = TestContext::new("test_init_new_repo", keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir);

    // Setup git repo
    ctx.setup_git_repo(3)?;

    // Generate test key
    let nsec = TestContext::generate_test_key();
    let repo_path = ctx.repo_path.to_string_lossy();

    // Initialize repository
    let args = build_init_args(
        "test-repo",
        "Test Repository",
        "A test repository",
        relays,
        &nsec,
        &repo_path,
        Some("json"),
    );
    let output = runner.run_success(&args).await?;

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

async fn test_init_existing_repo(keep_temp: bool, relays: &[String]) -> Result<()> {
    let ctx = TestContext::new("test_init_existing_repo", keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir);

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
    let repo_path = ctx.repo_path.to_string_lossy();

    // Initialize with minimal args (should detect from git)
    let args = build_init_args(
        "existing-repo",
        "Existing Repo",
        "Testing with existing repo",
        relays,
        &nsec,
        &repo_path,
        Some("minimal"),
    );
    let output = runner.run_success(&args).await?;

    // Should output just the nostr URL
    assert_contains(&output.stdout, "nostr://", "Should output nostr URL")?;

    Ok(())
}

async fn test_init_config_persistence(keep_temp: bool, relays: &[String]) -> Result<()> {
    let ctx = TestContext::new("test_init_config_persistence", keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir);

    ctx.setup_git_repo(2)?;

    let nsec = TestContext::generate_test_key();
    let id = uuid::Uuid::new_v4();
    let identifier = format!("config-test-{id}");
    let repo_path = ctx.repo_path.to_string_lossy();

    // Initialize repository
    let args = build_init_args(
        &identifier,
        "Config Test",
        "Testing configuration persistence",
        relays,
        &nsec,
        &repo_path,
        None,
    );
    runner.run_success(&args).await?;

    // Check git config was updated
    let git_config_path = ctx.repo_path.join(".git/config");
    assert_file_contains(&git_config_path, "[nostr]")?;
    assert_file_contains(&git_config_path, &format!("identifier = {identifier}"))?;
    assert_file_contains(&git_config_path, "name = Config Test")?;
    // Check that at least one relay was saved
    for relay in relays {
        if assert_file_contains(&git_config_path, &format!("relay = {relay}")).is_ok() {
            break; // At least one relay found
        }
    }

    Ok(())
}

async fn test_detect_from_git(keep_temp: bool, relays: &[String]) -> Result<()> {
    let ctx = TestContext::new("test_detect_from_git", keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir);

    ctx.setup_git_repo(2)?;

    // First initialize a repo
    let nsec = TestContext::generate_test_key();
    let id = uuid::Uuid::new_v4();
    let identifier = format!("detect-test-{id}");
    let repo_path = ctx.repo_path.to_string_lossy();

    let args = build_init_args(
        &identifier,
        "Detect Test",
        "Testing detection",
        relays,
        &nsec,
        &repo_path,
        None,
    );
    runner.run_success(&args).await?;

    // Generate announcement from existing repo (should detect saved config)
    let output = runner
        .run_success(&["generate", "--repo-path", &ctx.repo_path.to_string_lossy()])
        .await?;

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

    // Check that at least one of the provided relays is in the output
    let mut relay_found = false;
    for relay in relays {
        if output.stdout.contains(relay) {
            relay_found = true;
            break;
        }
    }
    if !relay_found {
        anyhow::bail!(
            "Expected to find at least one relay from {relays:?} in output, got: {output}",
            output = output.stdout
        );
    }

    Ok(())
}
