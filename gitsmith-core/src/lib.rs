pub mod account;
pub mod events;
pub mod patches;
pub mod pull_request;
pub mod repo;
pub mod types;

// Re-export main types and functions for convenience
pub use events::{
    KIND_GIT_PATCH, KIND_GIT_REPO_ANNOUNCEMENT, KIND_GIT_STATE, build_announcement_event,
    build_state_event,
};
pub use repo::{
    announce_repository, detect_from_git, get_git_state, update_git_config, update_git_config_full,
};
pub use types::{GitState, PublishConfig, PublishResult, RepoAnnouncement};
