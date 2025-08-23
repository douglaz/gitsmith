use serde::{Deserialize, Serialize};

/// Pull request structure matching the output from gitsmith list command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub created_at: u64,
    pub updated_at: Option<u64>,
    pub patches_count: usize,
    pub root_commit: Option<String>,
    pub status: String, // Using String for simplicity in tests
}

impl PullRequest {
    /// Find a PR by title in a list
    pub fn find_by_title<'a>(prs: &'a [PullRequest], title: &str) -> Option<&'a PullRequest> {
        prs.iter().find(|pr| pr.title == title)
    }
    
    /// Find PRs by author
    pub fn find_by_author<'a>(prs: &'a [PullRequest], author: &str) -> Vec<&'a PullRequest> {
        prs.iter().filter(|pr| pr.author == author).collect()
    }
}