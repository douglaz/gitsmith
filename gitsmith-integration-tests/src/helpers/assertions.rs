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
pub fn assert_json_contains(json_str: &str, expected_fields: &[&str]) -> Result<()> {
    let value: Value = serde_json::from_str(json_str)
        .with_context(|| format!("Failed to parse JSON: {}", json_str))?;
    
    for field in expected_fields {
        if !value.get(field).is_some() {
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