use crate::helpers::{GitsmithRunner, TestContext, assert_pr_details, assert_pr_exists};
use anyhow::Result;
use colored::*;
use tracing::{debug, info};

/// Generate a unique identifier for tests to avoid conflicts on public relays
fn generate_unique_identifier(prefix: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}-{}", prefix, timestamp)
}

/// Run all pull request workflow tests
pub async fn run_tests(keep_temp: bool, relays: &[String]) -> Result<(usize, usize)> {
    let mut passed = 0;
    let mut failed = 0;

    // Test sending a simple PR
    match test_send_pr_simple(keep_temp, relays).await {
        Ok(_) => {
            println!("  {check} test_send_pr_simple", check = "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!(
                "  {cross} test_send_pr_simple: {error}",
                cross = "✗".red(),
                error = e
            );
            failed += 1;
        }
    }

    // Test sending PR with title and description
    match test_send_pr_with_title_description(keep_temp, relays).await {
        Ok(_) => {
            println!(
                "  {check} test_send_pr_with_title_description",
                check = "✓".green()
            );
            passed += 1;
        }
        Err(e) => {
            println!(
                "  {cross} test_send_pr_with_title_description: {error}",
                cross = "✗".red(),
                error = e
            );
            failed += 1;
        }
    }

    // Test sending PR with no commits
    match test_send_pr_no_commits(keep_temp, relays).await {
        Ok(_) => {
            println!("  {check} test_send_pr_no_commits", check = "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!(
                "  {cross} test_send_pr_no_commits: {error}",
                cross = "✗".red(),
                error = e
            );
            failed += 1;
        }
    }

    // Test sending PR with multiple patches
    match test_send_pr_multiple_patches(keep_temp, relays).await {
        Ok(_) => {
            println!(
                "  {check} test_send_pr_multiple_patches",
                check = "✓".green()
            );
            passed += 1;
        }
        Err(e) => {
            println!(
                "  {cross} test_send_pr_multiple_patches: {error}",
                cross = "✗".red(),
                error = e
            );
            failed += 1;
        }
    }

    // Test full PR workflow
    match test_full_pr_workflow(keep_temp, relays).await {
        Ok(_) => {
            println!("  {check} test_full_pr_workflow", check = "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!(
                "  {cross} test_full_pr_workflow: {error}",
                cross = "✗".red(),
                error = e
            );
            failed += 1;
        }
    }

    // Test multiple PRs
    match test_multiple_prs(keep_temp, relays).await {
        Ok(_) => {
            println!("  {check} test_multiple_prs", check = "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!(
                "  {cross} test_multiple_prs: {error}",
                cross = "✗".red(),
                error = e
            );
            failed += 1;
        }
    }

    Ok((passed, failed))
}

async fn test_send_pr_simple(keep_temp: bool, relays: &[String]) -> Result<()> {
    let ctx = TestContext::new("test_send_pr_simple", keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir);

    // Setup repo with commits
    ctx.setup_git_repo(3)?;

    // Setup account
    let nsec = TestContext::generate_test_key();
    runner
        .run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])
        .await?;

    // Generate unique identifier to avoid conflicts
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let identifier = format!("pr-test-{}", timestamp);

    // Initialize repo
    // Build init command with dynamic relays
    let mut init_args = vec![
        "init",
        "--identifier",
        &identifier,
        "--name",
        "PR Test Repo",
        "--description",
        "Testing PRs",
        "--nsec",
        &nsec,
    ];
    for relay in relays {
        init_args.push("--relay");
        init_args.push(relay);
    }
    let repo_path = ctx.repo_path.to_string_lossy();
    init_args.push("--repo-path");
    init_args.push(&repo_path);
    runner.run_success(&init_args).await?;

    // Send PR
    let _output = runner
        .run_success(&[
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
        ])
        .await?;

    // We no longer check stderr - just verify the command succeeded
    // The actual verification happens when we list and check the PR exists

    // List PRs with retry to handle propagation delays
    let prs = crate::helpers::list_prs_with_retry(
        &runner,
        &ctx.repo_path.to_string_lossy(),
        10, // max retries - patient for public relays
    )
    .await?;

    // Verify we have exactly one PR
    if prs.is_empty() {
        anyhow::bail!("No PRs found after sending. The PR was not actually created!");
    }

    // Find and verify our PR
    let pr = assert_pr_exists(&prs, "Test PR")?;
    assert_pr_details(pr, "Test PR", "This is a test PR", 1)?;

    {
        println!("    ✓ Verified PR exists with correct details");
    }

    Ok(())
}

async fn test_send_pr_with_title_description(keep_temp: bool, relays: &[String]) -> Result<()> {
    let ctx = TestContext::new("test_send_pr_title_desc", keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir);

    ctx.setup_git_repo(5)?;

    let nsec = TestContext::generate_test_key();
    runner
        .run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])
        .await?;

    // Generate unique identifier to avoid conflicts
    let identifier = generate_unique_identifier("pr-title-test");

    // Build init command with dynamic relays
    let mut init_args = vec![
        "init",
        "--identifier",
        &identifier,
        "--name",
        "PR Title Test",
        "--description",
        "Testing with title/desc",
        "--nsec",
        &nsec,
    ];
    for relay in relays {
        init_args.push("--relay");
        init_args.push(relay);
    }
    let repo_path = ctx.repo_path.to_string_lossy();
    init_args.push("--repo-path");
    init_args.push(&repo_path);
    runner.run_success(&init_args).await?;

    // Send PR with specific title and description
    let _output = runner
        .run_success(&[
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
        ])
        .await?;

    // Command success is verified by run_success
    // The actual verification happens when we list and check the PR

    // List PRs with retry to handle propagation delays
    let prs = crate::helpers::list_prs_with_retry(
        &runner,
        &ctx.repo_path.to_string_lossy(),
        10, // max retries - patient for public relays
    )
    .await?;
    let pr = assert_pr_exists(&prs, "Feature: Add new functionality")?;

    // Note: The description might be modified when stored, so we check if it contains key parts
    if !pr.description.contains("Feature A") || !pr.description.contains("Feature B") {
        anyhow::bail!(
            "PR description doesn't contain expected features. Got: '{}'",
            pr.description
        );
    }

    if pr.patches_count != 2 {
        anyhow::bail!("Expected 2 patches, got {}", pr.patches_count);
    }

    {
        println!("    ✓ Verified PR with custom title/description");
    }

    Ok(())
}

async fn test_send_pr_no_commits(keep_temp: bool, relays: &[String]) -> Result<()> {
    let ctx = TestContext::new("test_send_pr_no_commits", keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir);

    // Setup repo with only 1 commit
    ctx.setup_git_repo(1)?;

    let nsec = TestContext::generate_test_key();
    runner
        .run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])
        .await?;

    // Generate unique identifier to avoid conflicts
    let identifier = generate_unique_identifier("pr-no-commits");

    // Build init command with dynamic relays
    let mut init_args = vec![
        "init",
        "--identifier",
        &identifier,
        "--name",
        "No Commits Test",
        "--description",
        "Testing with no commits to send",
        "--nsec",
        &nsec,
    ];
    for relay in relays {
        init_args.push("--relay");
        init_args.push(relay);
    }
    let repo_path = ctx.repo_path.to_string_lossy();
    init_args.push("--repo-path");
    init_args.push(&repo_path);
    runner.run_success(&init_args).await?;

    // Try to send PR from HEAD~1 (should fail as there's only 1 commit)
    let _output = runner
        .run_failure(&[
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
        ])
        .await?;

    // The failure is already verified by run_failure
    // We don't need to check the specific error message

    Ok(())
}

async fn test_send_pr_multiple_patches(keep_temp: bool, relays: &[String]) -> Result<()> {
    let ctx = TestContext::new("test_send_pr_multiple", keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir);

    // Create repo with many commits
    ctx.setup_git_repo(10)?;

    let nsec = TestContext::generate_test_key();
    runner
        .run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])
        .await?;

    // Generate unique identifier to avoid conflicts
    let identifier = generate_unique_identifier("pr-multiple");

    // Build init command with dynamic relays
    let mut init_args = vec![
        "init",
        "--identifier",
        &identifier,
        "--name",
        "Multiple Patches Test",
        "--description",
        "Testing with multiple patches",
        "--nsec",
        &nsec,
    ];
    for relay in relays {
        init_args.push("--relay");
        init_args.push(relay);
    }
    let repo_path = ctx.repo_path.to_string_lossy();
    init_args.push("--repo-path");
    init_args.push(&repo_path);
    runner.run_success(&init_args).await?;

    // Send PR with 5 patches
    let _output = runner
        .run_success(&[
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
        ])
        .await?;

    // Command success is verified by run_success
    // The actual verification happens when we check the PR has 5 patches

    // List PRs with retry to handle propagation delays
    info!("Listing PRs with retry for multi-patch PR");
    let prs = crate::helpers::list_prs_with_retry(
        &runner,
        &ctx.repo_path.to_string_lossy(),
        10, // max retries - patient for public relays
    )
    .await?;
    debug!("Found {} PRs", prs.len());

    info!("Verifying multi-patch PR details");
    let pr = assert_pr_exists(&prs, "Multiple commits PR")?;
    assert_pr_details(
        pr,
        "Multiple commits PR",
        "This PR contains multiple patches",
        5,
    )?;

    {
        println!("    ✓ Verified PR with 5 patches");
    }

    info!("test_send_pr_multiple_patches completed successfully");
    Ok(())
}

async fn test_full_pr_workflow(keep_temp: bool, relays: &[String]) -> Result<()> {
    let ctx = TestContext::new("test_full_pr_workflow", keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir);

    // Setup repository with many commits for a comprehensive test
    ctx.setup_git_repo(10)?;

    let nsec = TestContext::generate_test_key();
    runner
        .run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])
        .await?;

    // Generate unique identifier to avoid conflicts
    let identifier = generate_unique_identifier("workflow-test");

    // Initialize repo
    // Build init command with dynamic relays
    let mut init_args = vec![
        "init",
        "--identifier",
        &identifier,
        "--name",
        "Workflow Test Repo",
        "--description",
        "Testing complete PR workflow",
        "--nsec",
        &nsec,
    ];
    for relay in relays {
        init_args.push("--relay");
        init_args.push(relay);
    }
    let repo_path = ctx.repo_path.to_string_lossy();
    init_args.push("--repo-path");
    init_args.push(&repo_path);
    runner.run_success(&init_args).await?;

    // Step 1: Verify empty list initially
    let output = runner
        .run_success(&[
            "list",
            "--repo-path",
            &ctx.repo_path.to_string_lossy(),
            "--json",
        ])
        .await?;
    let prs = output.parse_pr_list()?;
    if !prs.is_empty() {
        anyhow::bail!("Expected empty PR list initially, got {} PRs", prs.len());
    }

    // Step 2: Send first PR
    runner
        .run_success(&[
            "send",
            "--title",
            "First PR",
            "--description",
            "Initial feature implementation",
            "--repo-path",
            &ctx.repo_path.to_string_lossy(),
            "--password",
            "test",
            "HEAD~3",
        ])
        .await?;

    // Step 3: Verify first PR exists with retry
    let prs = crate::helpers::list_prs_with_retry(
        &runner,
        &ctx.repo_path.to_string_lossy(),
        10, // max retries - patient for public relays
    )
    .await?;
    if prs.len() != 1 {
        anyhow::bail!("Expected 1 PR after first send, got {}", prs.len());
    }
    let first_pr = assert_pr_exists(&prs, "First PR")?;
    assert_pr_details(first_pr, "First PR", "Initial feature implementation", 3)?;

    // Step 4: Send update to the same PR (reply to it)
    // Note: This would require the --in-reply-to flag with the PR's event ID
    // For now, we'll send another independent PR

    // Step 5: Send second PR
    runner
        .run_success(&[
            "send",
            "--title",
            "Second PR",
            "--description",
            "Bug fixes",
            "--repo-path",
            &ctx.repo_path.to_string_lossy(),
            "--password",
            "test",
            "HEAD~5",
        ])
        .await?;

    // Step 6: Verify both PRs exist with retry
    let prs = crate::helpers::list_prs_with_retry(
        &runner,
        &ctx.repo_path.to_string_lossy(),
        10, // max retries - patient for public relays
    )
    .await?;
    if prs.len() != 2 {
        anyhow::bail!("Expected 2 PRs after second send, got {}", prs.len());
    }

    // Verify both PRs are present
    assert_pr_exists(&prs, "First PR")?;
    let second_pr = assert_pr_exists(&prs, "Second PR")?;
    if second_pr.patches_count != 5 {
        anyhow::bail!(
            "Second PR should have 5 patches, got {}",
            second_pr.patches_count
        );
    }

    {
        println!("    ✓ Complete workflow test passed");
        println!("    ✓ Created and verified 2 PRs");
    }

    Ok(())
}

async fn test_multiple_prs(keep_temp: bool, relays: &[String]) -> Result<()> {
    let ctx = TestContext::new("test_multiple_prs", keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir);

    // Setup repository with enough commits for multiple PRs
    ctx.setup_git_repo(8)?;

    let nsec = TestContext::generate_test_key();
    runner
        .run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])
        .await?;

    // Generate unique identifier to avoid conflicts
    let identifier = generate_unique_identifier("multi-pr-test");

    // Build init command with dynamic relays
    let mut init_args = vec![
        "init",
        "--identifier",
        &identifier,
        "--name",
        "Multiple PRs Test",
        "--description",
        "Testing multiple simultaneous PRs",
        "--nsec",
        &nsec,
    ];
    for relay in relays {
        init_args.push("--relay");
        init_args.push(relay);
    }
    let repo_path = ctx.repo_path.to_string_lossy();
    init_args.push("--repo-path");
    init_args.push(&repo_path);
    runner.run_success(&init_args).await?;

    // Send multiple PRs
    let pr_configs = vec![
        ("Feature A", "Implements feature A", "HEAD~1", 1),
        ("Feature B", "Implements feature B", "HEAD~2", 2),
        ("Feature C", "Implements feature C", "HEAD~3", 3),
    ];

    for (title, desc, range, _expected_patches) in &pr_configs {
        runner
            .run_success(&[
                "send",
                "--title",
                title,
                "--description",
                desc,
                "--repo-path",
                &ctx.repo_path.to_string_lossy(),
                "--password",
                "test",
                range,
            ])
            .await?;
    }

    // List and verify all PRs with retry
    let prs = crate::helpers::list_prs_with_retry(
        &runner,
        &ctx.repo_path.to_string_lossy(),
        10, // max retries - patient for public relays
    )
    .await?;

    // Verify we have exactly 3 PRs
    if prs.len() != 3 {
        anyhow::bail!("Expected 3 PRs, got {}", prs.len());
    }

    // Verify each PR exists with correct details
    for (title, desc, _range, expected_patches) in pr_configs {
        let pr = assert_pr_exists(&prs, title)?;
        assert_pr_details(pr, title, desc, expected_patches)?;
    }

    // Verify all PRs have unique IDs
    let mut ids = prs.iter().map(|pr| &pr.id).collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    if ids.len() != 3 {
        anyhow::bail!("PRs don't have unique IDs");
    }

    {
        println!("    ✓ Successfully created and verified 3 PRs");
        println!("    ✓ All PRs have unique IDs");
    }

    Ok(())
}
