use anyhow::Result;
use gitsmith_core::{
    account, announce_repository, detect_from_git, patches, pull_request, repo, types,
};
use nostr_sdk::prelude::*;
use rmcp::{
    ErrorData as McpError, RoleServer,
    handler::server::ServerHandler,
    model::{
        CallToolRequestParam, CallToolResult, Content, ListToolsResult, PaginatedRequestParam,
        ServerCapabilities, ServerInfo, Tool,
    },
    schemars,
    service::{RequestContext, ServiceExt},
    tool,
    transport::stdio,
};
use serde::Deserialize;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// Configuration for the MCP server
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub transport: Transport,
    #[allow(dead_code)]
    pub host: String,
    #[allow(dead_code)]
    pub port: u16,
}

#[derive(Debug, Clone)]
pub enum Transport {
    Stdio,
    #[allow(dead_code)]
    Sse,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            transport: Transport::Stdio,
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}

/// The main MCP server for gitsmith
#[derive(Clone)]
pub struct GitSmithMcpServer {
    config: McpServerConfig,
    #[allow(dead_code)]
    state: Arc<Mutex<ServerState>>,
}

#[derive(Default)]
struct ServerState {
    // Add any shared state here if needed
}

// Account management tool requests

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AccountImportRequest {
    #[schemars(description = "Nostr private key (hex or nsec format)")]
    pub private_key: String,
    #[schemars(description = "Name for the account")]
    pub name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AccountExportRequest {
    #[schemars(description = "Password for decryption")]
    pub password: String,
}

// Repository tool requests
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RepoInitRequest {
    #[schemars(description = "Repository identifier (unique)")]
    pub identifier: String,
    #[schemars(description = "Repository name")]
    pub name: String,
    #[schemars(description = "Repository description")]
    pub description: String,
    #[schemars(description = "Clone URLs")]
    pub clone_urls: Vec<String>,
    #[schemars(description = "Nostr relay URLs")]
    pub relays: Vec<String>,
    #[schemars(description = "Private key (hex or nsec)")]
    pub private_key: String,
    #[schemars(description = "Repository path")]
    pub repo_path: Option<String>,
    #[schemars(description = "Root commit")]
    pub root_commit: Option<String>,
    #[schemars(description = "Additional maintainer npubs")]
    pub maintainers: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RepoDetectRequest {
    #[schemars(description = "Repository path")]
    pub repo_path: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RepoStateRequest {
    #[schemars(description = "Repository identifier")]
    pub identifier: String,
    #[schemars(description = "Repository path")]
    pub repo_path: Option<String>,
}

// Pull request tool requests
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PrSendRequest {
    #[schemars(description = "PR title")]
    pub title: String,
    #[schemars(description = "PR description")]
    pub description: String,
    #[schemars(description = "Commits to include (e.g., HEAD~1)")]
    pub since: Option<String>,
    #[schemars(description = "Repository path")]
    pub repo_path: Option<String>,
    #[schemars(description = "Password for account")]
    pub password: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PrListRequest {
    #[schemars(description = "Repository identifier")]
    pub identifier: Option<String>,
    #[schemars(description = "PR status filter")]
    #[allow(dead_code)]
    pub status: Option<String>,
    #[schemars(description = "Repository path")]
    pub repo_path: Option<String>,
    #[schemars(description = "Password for account")]
    pub password: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[allow(dead_code)]
pub struct PrSyncRequest {
    #[schemars(description = "PR event ID")]
    pub event_id: String,
    #[schemars(description = "Repository path")]
    pub repo_path: Option<String>,
    #[schemars(description = "Password for account")]
    pub password: String,
}

// Patch tool requests
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PatchSendRequest {
    #[schemars(description = "Patch title")]
    pub title: Option<String>,
    #[schemars(description = "Commits to include (e.g., HEAD~1)")]
    pub since: Option<String>,
    #[schemars(description = "Repository path")]
    pub repo_path: Option<String>,
    #[schemars(description = "Password for account")]
    pub password: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PatchGenerateRequest {
    #[schemars(description = "Commits to include (e.g., HEAD~3)")]
    pub since: Option<String>,
    #[schemars(description = "Repository path")]
    pub repo_path: Option<String>,
}

impl GitSmithMcpServer {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(ServerState::default())),
        }
    }

    /// Start the MCP server
    pub async fn run(self) -> Result<()> {
        // Initialize tracing only if not in test mode
        let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
        if log_level != "error" {
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::from_default_env()
                        .add_directive(tracing::Level::INFO.into()),
                )
                .init();

            info!("Starting gitsmith MCP server");
        }

        match self.config.transport {
            Transport::Stdio => {
                if log_level != "error" {
                    info!("Starting MCP server with stdio transport");
                }
                let service = self.serve(stdio()).await?;
                service.waiting().await?;
            }
            Transport::Sse => {
                anyhow::bail!("SSE transport not yet implemented");
            }
        }

        Ok(())
    }
}

// Implement the tool methods
impl GitSmithMcpServer {
    // Account management tools

    #[tool(description = "Import an existing Nostr account")]
    async fn account_import(&self, req: AccountImportRequest) -> CallToolResult {
        // The actual login function takes nsec/hex and password
        match account::login(&req.private_key, &req.name) {
            Ok(_) => CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "success": true,
                    "message": "Account imported and set as active"
                })
                .to_string(),
            )]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
        }
    }

    #[tool(description = "Logout from active account")]
    async fn account_logout(&self) -> CallToolResult {
        match account::logout() {
            Ok(_) => CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "success": true,
                    "message": "Logged out successfully"
                })
                .to_string(),
            )]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
        }
    }

    #[tool(description = "List all gitsmith accounts")]
    async fn account_list(&self) -> CallToolResult {
        match account::list_accounts() {
            Ok(accounts) => CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&accounts).unwrap_or_else(|e| e.to_string()),
            )]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
        }
    }

    #[tool(description = "Export active account private key")]
    async fn account_export(&self, req: AccountExportRequest) -> CallToolResult {
        match account::export_keys(&req.password) {
            Ok(nsec) => CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "success": true,
                    "nsec": nsec
                })
                .to_string(),
            )]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
        }
    }

    // Repository tools
    #[tool(description = "Initialize a repository on Nostr")]
    async fn repo_init(&self, req: RepoInitRequest) -> CallToolResult {
        // Parse private key
        let keys = match req.private_key.parse::<Keys>() {
            Ok(k) => k,
            Err(e) => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Invalid private key: {e}"
                ))]);
            }
        };

        let repo_path = req
            .repo_path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        // Detect root commit if not provided
        let root_commit = if let Some(rc) = req.root_commit {
            rc
        } else {
            // Detect from git will include root commit
            match detect_from_git(&repo_path) {
                Ok(detected) => detected.root_commit,
                Err(e) => {
                    return CallToolResult::error(vec![Content::text(format!(
                        "Failed to detect root commit: {e}"
                    ))]);
                }
            }
        };

        let announcement = types::RepoAnnouncement {
            identifier: req.identifier,
            name: req.name,
            description: req.description,
            clone_urls: req.clone_urls,
            relays: req.relays,
            web: vec![],
            root_commit,
            maintainers: req.maintainers.unwrap_or_default(),
            grasp_servers: vec![],
        };

        let config = types::PublishConfig {
            timeout_secs: 30,
            wait_for_send: true,
        };

        match announce_repository(announcement, &keys.secret_key().to_secret_hex(), config).await {
            Ok(result) => CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| e.to_string()),
            )]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
        }
    }

    #[tool(description = "Detect repository information from git")]
    async fn repo_detect(&self, req: RepoDetectRequest) -> CallToolResult {
        let repo_path = req
            .repo_path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        match detect_from_git(&repo_path) {
            Ok(announcement) => CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&announcement).unwrap_or_else(|e| e.to_string()),
            )]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
        }
    }

    #[tool(description = "Get repository state")]
    async fn repo_state(&self, req: RepoStateRequest) -> CallToolResult {
        let repo_path = req
            .repo_path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        match repo::get_git_state(&repo_path, &req.identifier) {
            Ok(state) => CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&state).unwrap_or_else(|e| e.to_string()),
            )]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
        }
    }

    // Pull request tools
    #[tool(description = "Send a pull request to Nostr")]
    async fn pr_send(&self, req: PrSendRequest) -> CallToolResult {
        // Get account keys
        let keys = match account::get_active_keys(&req.password) {
            Ok(k) => k,
            Err(e) => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Failed to get account keys: {e}"
                ))]);
            }
        };

        let repo_path = req
            .repo_path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        // Detect repository info
        let repo_announcement = match detect_from_git(&repo_path) {
            Ok(a) => a,
            Err(e) => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Failed to detect repository: {e}"
                ))]);
            }
        };

        // Generate patches
        let since = req.since.as_deref().unwrap_or("HEAD~1");
        let patches_list = match patches::generate_patches(&repo_path, Some(since), None) {
            Ok(p) => p,
            Err(e) => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Failed to generate patches: {e}"
                ))]);
            }
        };

        if patches_list.is_empty() {
            return CallToolResult::error(vec![Content::text("No patches to send".to_string())]);
        }

        // Create repository coordinate
        let repo_coordinate = format!(
            "30617:{pubkey}:{identifier}",
            pubkey = keys.public_key(),
            identifier = repo_announcement.identifier
        );

        // Create PR events
        let events = match patches::create_pull_request_event(
            &keys,
            &repo_coordinate,
            &req.title,
            &req.description,
            patches_list,
            &repo_announcement.root_commit,
            None,
        ) {
            Ok(e) => e,
            Err(e) => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Failed to create PR events: {e}"
                ))]);
            }
        };

        // Send to relays
        if repo_announcement.relays.is_empty() {
            return CallToolResult::error(vec![Content::text(
                "No relays configured for repository".to_string(),
            )]);
        }

        let client = Client::new(keys.clone());
        for relay_url in &repo_announcement.relays {
            if let Err(e) = client.add_relay(relay_url).await {
                tracing::warn!("Failed to add relay {}: {}", relay_url, e);
            }
        }

        client.connect().await;

        let mut successes = vec![];
        let mut failures = vec![];

        for event in events {
            match client.send_event(&event).await {
                Ok(output) => {
                    for relay in output.success {
                        successes.push(relay.to_string());
                    }
                    for (relay, msg) in output.failed {
                        failures.push(format!("{}: {}", relay, msg));
                    }
                }
                Err(e) => failures.push(format!("Failed to send event: {e}")),
            }
        }

        CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "success": true,
                "successes": successes,
                "failures": failures,
                "message": format!("PR sent to {} relay(s)", successes.len())
            })
            .to_string(),
        )])
    }

    #[tool(description = "List pull requests")]
    async fn pr_list(&self, req: PrListRequest) -> CallToolResult {
        // Get account keys
        let keys = match account::get_active_keys(&req.password) {
            Ok(k) => k,
            Err(e) => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Failed to get account keys: {e}"
                ))]);
            }
        };

        let repo_path = req
            .repo_path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        // Get repository info
        let repo_announcement = match detect_from_git(&repo_path) {
            Ok(a) => a,
            Err(e) => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Failed to detect repository: {e}"
                ))]);
            }
        };

        // Get repository identifier from request or detection
        let identifier = req.identifier.unwrap_or(repo_announcement.identifier);

        // Create repository coordinate
        let repo_coordinate = format!(
            "30617:{pubkey}:{identifier}",
            pubkey = keys.public_key(),
            identifier = identifier
        );

        // List PRs
        match pull_request::list_pull_requests(&repo_coordinate, repo_announcement.relays).await {
            Ok(prs) => CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&prs).unwrap_or_else(|e| e.to_string()),
            )]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
        }
    }

    // Patch tools
    #[tool(description = "Send patches to Nostr")]
    async fn patch_send(&self, req: PatchSendRequest) -> CallToolResult {
        // Get account keys
        let keys = match account::get_active_keys(&req.password) {
            Ok(k) => k,
            Err(e) => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Failed to get account keys: {e}"
                ))]);
            }
        };

        let repo_path = req
            .repo_path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        // Detect repository info
        let repo_announcement = match detect_from_git(&repo_path) {
            Ok(a) => a,
            Err(e) => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Failed to detect repository: {e}"
                ))]);
            }
        };

        // Generate patches
        let since = req.since.as_deref().unwrap_or("HEAD~1");
        let patches_list = match patches::generate_patches(&repo_path, Some(since), None) {
            Ok(p) => p,
            Err(e) => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Failed to generate patches: {e}"
                ))]);
            }
        };

        if patches_list.is_empty() {
            return CallToolResult::error(vec![Content::text("No patches to send".to_string())]);
        }

        // Send patches as individual events
        let client = Client::new(keys.clone());
        for relay_url in &repo_announcement.relays {
            if let Err(e) = client.add_relay(relay_url).await {
                tracing::warn!("Failed to add relay {}: {}", relay_url, e);
            }
        }

        client.connect().await;

        let mut successes = vec![];
        let mut failures = vec![];

        for patch in patches_list {
            // Create patch event (simplified - you may need to implement proper patch event creation)
            let content = serde_json::json!({
                "title": req.title.clone().unwrap_or_else(|| "Patch".to_string()),
                "patch": patch,
            })
            .to_string();

            let event_builder = EventBuilder::text_note(content);
            let event = match event_builder.sign_with_keys(&keys) {
                Ok(e) => e,
                Err(e) => {
                    failures.push(format!("Failed to sign event: {e}"));
                    continue;
                }
            };

            match client.send_event(&event).await {
                Ok(output) => {
                    for relay in output.success {
                        successes.push(relay.to_string());
                    }
                    for (relay, msg) in output.failed {
                        failures.push(format!("{}: {}", relay, msg));
                    }
                }
                Err(e) => failures.push(format!("Failed to send event: {e}")),
            }
        }

        CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "success": true,
                "successes": successes,
                "failures": failures,
                "message": format!("Patches sent to {} relay(s)", successes.len())
            })
            .to_string(),
        )])
    }

    #[tool(description = "Generate patches from git commits")]
    async fn patch_generate(&self, req: PatchGenerateRequest) -> CallToolResult {
        let repo_path = req
            .repo_path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        let since = req.since.as_deref().unwrap_or("HEAD~1");

        match patches::generate_patches(&repo_path, Some(since), None) {
            Ok(patches_list) => CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "patches": patches_list,
                    "count": patches_list.len()
                })
                .to_string(),
            )]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
        }
    }
}

// Helper function to create a Tool with proper types
fn create_tool(name: &'static str, description: &'static str, schema: serde_json::Value) -> Tool {
    Tool {
        name: name.into(),
        description: Some(description.into()),
        input_schema: Arc::new(schema.as_object().unwrap().clone()),
        output_schema: None,
        annotations: None,
    }
}

// Implement ServerHandler trait
impl ServerHandler for GitSmithMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let tools = vec![
            // Account tools
            create_tool(
                "account_import",
                "Import an existing Nostr account",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "private_key": {
                            "type": "string",
                            "description": "Nostr private key (hex or nsec format)"
                        },
                        "name": {
                            "type": "string",
                            "description": "Name for the account"
                        }
                    },
                    "required": ["private_key", "name"]
                }),
            ),
            create_tool(
                "account_logout",
                "Logout from active account",
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            ),
            create_tool(
                "account_list",
                "List all gitsmith accounts",
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            ),
            create_tool(
                "account_export",
                "Export active account private key",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "password": {
                            "type": "string",
                            "description": "Password for decryption"
                        }
                    },
                    "required": ["password"]
                }),
            ),
            // Repository tools
            create_tool(
                "repo_init",
                "Initialize a repository on Nostr",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "identifier": {
                            "type": "string",
                            "description": "Repository identifier (unique)"
                        },
                        "name": {
                            "type": "string",
                            "description": "Repository name"
                        },
                        "description": {
                            "type": "string",
                            "description": "Repository description"
                        },
                        "clone_urls": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Clone URLs"
                        },
                        "relays": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Nostr relay URLs"
                        },
                        "private_key": {
                            "type": "string",
                            "description": "Private key (hex or nsec)"
                        },
                        "repo_path": {
                            "type": "string",
                            "description": "Repository path"
                        },
                        "root_commit": {
                            "type": "string",
                            "description": "Root commit"
                        },
                        "maintainers": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Additional maintainer npubs"
                        }
                    },
                    "required": ["identifier", "name", "description", "clone_urls", "relays", "private_key"]
                }),
            ),
            create_tool(
                "repo_detect",
                "Detect repository information from git",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "repo_path": {
                            "type": "string",
                            "description": "Repository path"
                        }
                    }
                }),
            ),
            create_tool(
                "repo_state",
                "Get repository state",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "identifier": {
                            "type": "string",
                            "description": "Repository identifier"
                        },
                        "repo_path": {
                            "type": "string",
                            "description": "Repository path"
                        }
                    },
                    "required": ["identifier"]
                }),
            ),
            // Pull request tools
            create_tool(
                "pr_send",
                "Send a pull request to Nostr",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "PR title"
                        },
                        "description": {
                            "type": "string",
                            "description": "PR description"
                        },
                        "since": {
                            "type": "string",
                            "description": "Commits to include (e.g., HEAD~1)"
                        },
                        "repo_path": {
                            "type": "string",
                            "description": "Repository path"
                        },
                        "password": {
                            "type": "string",
                            "description": "Password for account"
                        }
                    },
                    "required": ["title", "description", "password"]
                }),
            ),
            create_tool(
                "pr_list",
                "List pull requests",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "identifier": {
                            "type": "string",
                            "description": "Repository identifier"
                        },
                        "status": {
                            "type": "string",
                            "description": "PR status filter"
                        },
                        "repo_path": {
                            "type": "string",
                            "description": "Repository path"
                        },
                        "password": {
                            "type": "string",
                            "description": "Password for account"
                        }
                    },
                    "required": ["password"]
                }),
            ),
            // Patch tools
            create_tool(
                "patch_send",
                "Send patches to Nostr",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Patch title"
                        },
                        "since": {
                            "type": "string",
                            "description": "Commits to include (e.g., HEAD~1)"
                        },
                        "repo_path": {
                            "type": "string",
                            "description": "Repository path"
                        },
                        "password": {
                            "type": "string",
                            "description": "Password for account"
                        }
                    },
                    "required": ["password"]
                }),
            ),
            create_tool(
                "patch_generate",
                "Generate patches from git commits",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "since": {
                            "type": "string",
                            "description": "Commits to include (e.g., HEAD~3)"
                        },
                        "repo_path": {
                            "type": "string",
                            "description": "Repository path"
                        }
                    }
                }),
            ),
        ];

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let args = request.arguments.unwrap_or_default();

        match request.name.as_ref() {
            "account_login" => {
                // Login is not really usable via MCP since it needs the actual nsec
                Ok(CallToolResult::error(vec![Content::text(
                    "Login requires nsec/hex key. Use account_import instead".to_string(),
                )]))
            }
            "account_import" => {
                let private_key = args
                    .get("private_key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        McpError::invalid_request("private_key parameter required", None)
                    })?;
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::invalid_request("name parameter required", None))?;
                Ok(self
                    .account_import(AccountImportRequest {
                        private_key: private_key.to_string(),
                        name: name.to_string(),
                    })
                    .await)
            }
            "account_logout" => Ok(self.account_logout().await),
            "account_list" => Ok(self.account_list().await),
            "account_export" => {
                let password = args
                    .get("password")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        McpError::invalid_request("password parameter required", None)
                    })?;
                Ok(self
                    .account_export(AccountExportRequest {
                        password: password.to_string(),
                    })
                    .await)
            }
            "repo_init" => {
                let req = RepoInitRequest {
                    identifier: args
                        .get("identifier")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            McpError::invalid_request("identifier parameter required", None)
                        })?
                        .to_string(),
                    name: args
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::invalid_request("name parameter required", None))?
                        .to_string(),
                    description: args
                        .get("description")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            McpError::invalid_request("description parameter required", None)
                        })?
                        .to_string(),
                    clone_urls: args
                        .get("clone_urls")
                        .and_then(|v| v.as_array())
                        .ok_or_else(|| {
                            McpError::invalid_request("clone_urls parameter required", None)
                        })?
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect(),
                    relays: args
                        .get("relays")
                        .and_then(|v| v.as_array())
                        .ok_or_else(|| {
                            McpError::invalid_request("relays parameter required", None)
                        })?
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect(),
                    private_key: args
                        .get("private_key")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            McpError::invalid_request("private_key parameter required", None)
                        })?
                        .to_string(),
                    repo_path: args
                        .get("repo_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    root_commit: args
                        .get("root_commit")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    maintainers: args
                        .get("maintainers")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        }),
                };
                Ok(self.repo_init(req).await)
            }
            "repo_detect" => {
                let repo_path = args
                    .get("repo_path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Ok(self.repo_detect(RepoDetectRequest { repo_path }).await)
            }
            "repo_state" => {
                let identifier =
                    args.get("identifier")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            McpError::invalid_request("identifier parameter required", None)
                        })?;
                let repo_path = args
                    .get("repo_path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Ok(self
                    .repo_state(RepoStateRequest {
                        identifier: identifier.to_string(),
                        repo_path,
                    })
                    .await)
            }
            "pr_send" => {
                let req = PrSendRequest {
                    title: args
                        .get("title")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::invalid_request("title parameter required", None))?
                        .to_string(),
                    description: args
                        .get("description")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            McpError::invalid_request("description parameter required", None)
                        })?
                        .to_string(),
                    since: args
                        .get("since")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    repo_path: args
                        .get("repo_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    password: args
                        .get("password")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            McpError::invalid_request("password parameter required", None)
                        })?
                        .to_string(),
                };
                Ok(self.pr_send(req).await)
            }
            "pr_list" => {
                let req = PrListRequest {
                    identifier: args
                        .get("identifier")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    status: args
                        .get("status")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    repo_path: args
                        .get("repo_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    password: args
                        .get("password")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            McpError::invalid_request("password parameter required", None)
                        })?
                        .to_string(),
                };
                Ok(self.pr_list(req).await)
            }
            "patch_send" => {
                let req = PatchSendRequest {
                    title: args
                        .get("title")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    since: args
                        .get("since")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    repo_path: args
                        .get("repo_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    password: args
                        .get("password")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            McpError::invalid_request("password parameter required", None)
                        })?
                        .to_string(),
                };
                Ok(self.patch_send(req).await)
            }
            "patch_generate" => {
                let req = PatchGenerateRequest {
                    since: args
                        .get("since")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    repo_path: args
                        .get("repo_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                };
                Ok(self.patch_generate(req).await)
            }
            _ => Err(McpError::invalid_request(
                format!("Tool '{}' not found", request.name),
                None,
            )),
        }
    }
}
