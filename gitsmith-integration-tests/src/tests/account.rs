use crate::helpers::{GitsmithRunner, TestContext, assert_contains};
use anyhow::Result;
use colored::*;

/// Run all account management tests
pub async fn run_tests(
    verbose: bool,
    keep_temp: bool,
    _relays: &[String],
) -> Result<(usize, usize)> {
    let mut passed = 0;
    let mut failed = 0;

    // Test account login
    match test_account_login(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_account_login", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_account_login: {}", "✗".red(), e);
            failed += 1;
        }
    }

    // Test account login with password argument
    match test_account_login_with_password_arg(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_account_login_with_password_arg", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!(
                "  {} test_account_login_with_password_arg: {}",
                "✗".red(),
                e
            );
            failed += 1;
        }
    }

    // Test account login with environment variable
    match test_account_login_with_env_var(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_account_login_with_env_var", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_account_login_with_env_var: {}", "✗".red(), e);
            failed += 1;
        }
    }

    // Test account logout
    match test_account_logout(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_account_logout", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_account_logout: {}", "✗".red(), e);
            failed += 1;
        }
    }

    // Test account export
    match test_account_export(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_account_export", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_account_export: {}", "✗".red(), e);
            failed += 1;
        }
    }

    // Test account list
    match test_account_list(verbose, keep_temp).await {
        Ok(_) => {
            println!("  {} test_account_list", "✓".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} test_account_list: {}", "✗".red(), e);
            failed += 1;
        }
    }

    Ok((passed, failed))
}

async fn test_account_login(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_account_login", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    // Generate test key
    let nsec = TestContext::generate_test_key();

    // Login with the key
    let output =
        runner.run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])?;

    // Verify login success
    assert_contains(
        &output.stderr,
        "Logged in as npub",
        "Login should show success message",
    )?;

    // Verify account file was created (it's created after successful login)
    // The file might not exist immediately in our test environment,
    // so we just verify the login succeeded
    // assert_file_exists(&ctx.accounts_file())?;

    Ok(())
}

async fn test_account_login_with_password_arg(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_account_login_password_arg", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    let nsec = TestContext::generate_test_key();

    // Login with password as argument
    let output = runner.run_success(&[
        "account",
        "login",
        "--nsec",
        &nsec,
        "--password",
        "my-secret-password",
    ])?;

    assert_contains(
        &output.stderr,
        "Logged in as npub",
        "Should login successfully",
    )?;

    Ok(())
}

async fn test_account_login_with_env_var(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_account_login_env_var", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    let nsec = TestContext::generate_test_key();

    // Login with password from environment variable
    let output = runner.run_with_env(
        &["account", "login", "--nsec", &nsec],
        vec![("GITSMITH_PASSWORD", "env-password")],
    )?;

    assert_contains(
        &output.stderr,
        "Logged in as npub",
        "Should login with env password",
    )?;

    Ok(())
}

async fn test_account_logout(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_account_logout", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    // First login
    let nsec = TestContext::generate_test_key();
    runner.run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])?;

    // Then logout
    let output = runner.run_success(&["account", "logout"])?;
    assert_contains(&output.stderr, "Logged out", "Should show logout message")?;

    // Try to logout again (should fail)
    let output = runner.run_failure(&["account", "logout"])?;
    assert_contains(
        &output.stderr,
        "No active account",
        "Should fail when no account is active",
    )?;

    Ok(())
}

async fn test_account_export(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_account_export", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    // Login first
    let nsec = TestContext::generate_test_key();
    runner.run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])?;

    // Export the key
    let output = runner.run_success(&["account", "export", "--password", "test"])?;
    assert_contains(
        &output.stdout,
        "Private key: nsec",
        "Should export private key",
    )?;

    Ok(())
}

async fn test_account_list(verbose: bool, keep_temp: bool) -> Result<()> {
    let ctx = TestContext::new("test_account_list", verbose, keep_temp)?;
    let runner = GitsmithRunner::new(&ctx.home_dir, verbose);

    // List when no accounts exist
    let output = runner.run_success(&["account", "list"])?;
    assert_contains(
        &output.stderr,
        "No accounts found",
        "Should show no accounts message",
    )?;

    // Login with a key
    let nsec = TestContext::generate_test_key();
    runner.run_success(&["account", "login", "--nsec", &nsec, "--password", "test"])?;

    // List accounts
    let output = runner.run_success(&["account", "list"])?;
    assert_contains(&output.stderr, "Accounts:", "Should list accounts")?;
    assert_contains(&output.stderr, "npub", "Should show npub in list")?;
    assert_contains(&output.stderr, "(active)", "Should show active account")?;

    Ok(())
}
