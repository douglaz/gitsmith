use anyhow::{Context, Result, bail};
use git2::Repository;
use nostr::{Event, EventBuilder, Keys, Kind, Tag, TagKind};
use std::path::Path;

/// Kind for patch events (NIP-34)
pub const KIND_PATCH: Kind = Kind::Custom(1617);
/// Kind for pull request events
pub const KIND_PULL_REQUEST: Kind = Kind::Custom(1618);
/// Kind for pull request updates
pub const KIND_PULL_REQUEST_UPDATE: Kind = Kind::Custom(1619);

/// Generate patches from git commits
pub fn generate_patches(
    repo_path: &Path,
    since_commit: Option<&str>,
    count: Option<usize>,
) -> Result<Vec<String>> {
    let repo = Repository::open(repo_path)?;

    // Get the commit range
    let head = repo.head()?.peel_to_commit()?;
    let mut commits = Vec::new();

    if let Some(since) = since_commit {
        // Parse the since commit
        let since_oid = if since.contains("~") {
            // Handle HEAD~N notation
            let parts: Vec<&str> = since.split('~').collect();
            if parts.len() != 2 || parts[0] != "HEAD" {
                bail!("Invalid commit reference: {since}");
            }
            let n: usize = parts[1]
                .parse()
                .with_context(|| format!("Invalid number in {since}"))?;

            // Walk back N commits from HEAD
            let mut current = head.clone();
            for _ in 0..n {
                if let Ok(parent) = current.parent(0) {
                    current = parent;
                } else {
                    bail!("Not enough commits for {since}");
                }
            }
            current.id()
        } else {
            // Try to parse as commit hash or reference
            repo.revparse_single(since)?.id()
        };

        // Walk from HEAD to since_commit
        let mut revwalk = repo.revwalk()?;
        revwalk.push(head.id())?;

        for oid in revwalk {
            let oid = oid?;
            if oid == since_oid {
                break;
            }
            commits.push(oid);
        }
    } else if let Some(n) = count {
        // Get last N commits
        let mut revwalk = repo.revwalk()?;
        revwalk.push(head.id())?;

        for (i, oid) in revwalk.enumerate() {
            if i >= n {
                break;
            }
            commits.push(oid?);
        }
    } else {
        // Default to last commit
        commits.push(head.id());
    }

    // Reverse to get chronological order
    commits.reverse();

    // Generate patches for each commit
    let mut patches = Vec::new();
    for oid in commits {
        let commit = repo.find_commit(oid)?;
        let patch = generate_patch_for_commit(&repo, &commit)?;
        patches.push(patch);
    }

    Ok(patches)
}

/// Generate a patch string for a single commit
fn generate_patch_for_commit(repo: &Repository, commit: &git2::Commit) -> Result<String> {
    let parent = if commit.parent_count() > 0 {
        Some(commit.parent(0)?)
    } else {
        None
    };

    let tree = commit.tree()?;
    let parent_tree = parent.as_ref().map(|p| p.tree()).transpose()?;

    let mut diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
    diff.find_similar(None)?;

    let mut patch = String::new();

    // Add commit header
    patch.push_str(&format!(
        "From {commit_id} Mon Sep 17 00:00:00 2001\n",
        commit_id = commit.id()
    ));
    patch.push_str(&format!(
        "From: {name} <{email}>\n",
        name = commit.author().name().unwrap_or("Unknown"),
        email = commit.author().email().unwrap_or("unknown@example.com")
    ));
    patch.push_str(&format!(
        "Date: {date}\n",
        date = chrono::DateTime::from_timestamp(commit.time().seconds(), 0)
            .map(|dt| dt.format("%a, %d %b %Y %H:%M:%S %z").to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    ));
    patch.push_str(&format!(
        "Subject: {subject}\n",
        subject = commit.summary().unwrap_or("No subject")
    ));
    patch.push('\n');

    // Add commit message body
    if let Some(msg) = commit.message() {
        let lines: Vec<&str> = msg.lines().collect();
        if lines.len() > 1 {
            for line in &lines[1..] {
                patch.push_str(line);
                patch.push('\n');
            }
            patch.push('\n');
        }
    }

    // Add diff
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        patch.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
        true
    })?;

    // Add footer
    patch.push_str("-- \n");
    patch.push_str("2.34.1\n");

    Ok(patch)
}

/// Create a pull request event
pub fn create_pull_request_event(
    keys: &Keys,
    repo_coordinate: &str,
    title: &str,
    description: &str,
    patches: Vec<String>,
    root_commit: &str,
    reply_to: Option<String>,
) -> Result<Vec<Event>> {
    let mut events = Vec::new();

    // Create patch events first
    let mut patch_event_ids = Vec::new();
    for (i, patch) in patches.iter().enumerate() {
        let mut tags = vec![Tag::custom(
            TagKind::Custom("alt".into()),
            vec![format!(
                "git patch: {current}/{total}",
                current = i + 1,
                total = patches.len()
            )],
        )];

        // Add reference to previous patch if not first
        if i > 0 {
            tags.push(Tag::event(patch_event_ids[i - 1]));
        }

        let patch_event = EventBuilder::new(KIND_PATCH, patch.clone())
            .tags(tags)
            .sign_with_keys(keys)?;

        patch_event_ids.push(patch_event.id);
        events.push(patch_event);
    }

    // Create the PR event
    let kind = if reply_to.is_some() {
        KIND_PULL_REQUEST_UPDATE
    } else {
        KIND_PULL_REQUEST
    };

    let mut pr_tags = vec![
        // Repository reference (NIP-33 a tag)
        Tag::custom(
            TagKind::Custom("a".into()),
            vec![repo_coordinate.to_string()],
        ),
        // Subject/title
        Tag::custom(TagKind::Custom("subject".into()), vec![title.to_string()]),
        // Root commit
        Tag::custom(TagKind::Custom("c".into()), vec![root_commit.to_string()]),
    ];

    // Add reference to patches
    if !patch_event_ids.is_empty() {
        pr_tags.push(Tag::event(patch_event_ids[0]));
    }

    // Add reply reference if updating
    if let Some(reply_id) = reply_to {
        pr_tags.push(Tag::custom(TagKind::Custom("e".into()), vec![reply_id]));
    }

    let pr_event = EventBuilder::new(kind, description)
        .tags(pr_tags)
        .sign_with_keys(keys)?;

    events.push(pr_event);

    Ok(events)
}

/// Parse a repository coordinate (e.g., "30617:pubkey:identifier")
pub fn parse_repo_coordinate(coordinate: &str) -> Result<(String, String, String)> {
    let parts: Vec<&str> = coordinate.split(':').collect();
    if parts.len() != 3 {
        bail!("Invalid repository coordinate format. Expected: kind:pubkey:identifier");
    }

    Ok((
        parts[0].to_string(),
        parts[1].to_string(),
        parts[2].to_string(),
    ))
}
