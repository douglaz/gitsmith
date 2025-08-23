use anyhow::{Context, Result};
use regex::Regex;
use serde_json::Value;

/// Assert that a string contains another string
pub fn assert_contains(haystack: &str, needle: &str, message: &str) -> Result<()> {
    if !haystack.contains(needle) {
        anyhow::bail!(
            "{}\nExpected to contain: '{}'\nActual: '{}'",
            message,
            needle,
            haystack
        );
    }
    Ok(())
}

/// Assert that a string does not contain another string
#[allow(dead_code)]
pub fn assert_not_contains(haystack: &str, needle: &str, message: &str) -> Result<()> {
    if haystack.contains(needle) {
        anyhow::bail!(
            "{}\nExpected NOT to contain: '{}'\nActual: '{}'",
            message,
            needle,
            haystack
        );
    }
    Ok(())
}

/// Assert that a string matches a regex pattern
#[allow(dead_code)]
pub fn assert_matches(text: &str, pattern: &str, message: &str) -> Result<()> {
    let re = Regex::new(pattern).context("Invalid regex pattern")?;
    if !re.is_match(text) {
        anyhow::bail!(
            "{}\nExpected to match pattern: '{}'\nActual: '{}'",
            message,
            pattern,
            text
        );
    }
    Ok(())
}

/// Assert that JSON contains expected fields
#[allow(dead_code)]
pub fn assert_json_contains(json_str: &str, expected_fields: &[&str]) -> Result<()> {
    let value: Value = serde_json::from_str(json_str)
        .with_context(|| format!("Failed to parse JSON: {}", json_str))?;

    for field in expected_fields {
        if value.get(field).is_none() {
            anyhow::bail!(
                "JSON missing expected field '{}'\nActual JSON: {}",
                field,
                json_str
            );
        }
    }

    Ok(())
}

/// Assert that a file exists
#[allow(dead_code)]
pub fn assert_file_exists(path: &std::path::Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("File does not exist: {}", path.display());
    }
    Ok(())
}

/// Assert that a file contains text
pub fn assert_file_contains(path: &std::path::Path, text: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    if !content.contains(text) {
        anyhow::bail!(
            "File {} does not contain expected text: '{}'",
            path.display(),
            text
        );
    }

    Ok(())
}

/// Assert that a PR with given title exists in the list
pub fn assert_pr_exists<'a>(
    prs: &'a [crate::helpers::PullRequest],
    title: &str,
) -> Result<&'a crate::helpers::PullRequest> {
    prs.iter()
        .find(|pr| pr.title == title)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "PR with title '{}' not found. Available PRs: {:?}",
                title,
                prs.iter().map(|pr| &pr.title).collect::<Vec<_>>()
            )
        })
}

/// Assert PR has expected details
pub fn assert_pr_details(
    pr: &crate::helpers::PullRequest,
    title: &str,
    description: &str,
    patches_count: usize,
) -> Result<()> {
    if pr.title != title {
        anyhow::bail!(
            "PR title mismatch. Expected: '{}', Got: '{}'",
            title,
            pr.title
        );
    }
    
    if pr.description != description {
        anyhow::bail!(
            "PR description mismatch. Expected: '{}', Got: '{}'",
            description,
            pr.description
        );
    }
    
    if pr.patches_count != patches_count {
        anyhow::bail!(
            "PR patches count mismatch. Expected: {}, Got: {}",
            patches_count,
            pr.patches_count
        );
    }
    
    Ok(())
}
