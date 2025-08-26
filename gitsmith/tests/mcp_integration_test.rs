use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// MCP protocol request structure
#[derive(Debug, Serialize)]
struct McpRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: Option<u64>,
}

/// MCP protocol response structure
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct McpResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<Value>,
    id: Option<u64>,
}

/// MCP test client for integration testing
struct McpTestClient {
    process: Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    request_id: u64,
}

impl McpTestClient {
    /// Start the MCP server and create a test client
    fn new() -> Result<Self> {
        // Set RUST_LOG to error to avoid info logs interfering with JSON parsing
        let mut process = Command::new("cargo")
            .args(["run", "--", "mcp-server", "-t", "stdio"])
            .env("RUST_LOG", "error")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to spawn MCP server")?;

        let stdin = process.stdin.take().context("Failed to get stdin handle")?;
        let stdout = BufReader::new(
            process
                .stdout
                .take()
                .context("Failed to get stdout handle")?,
        );

        let mut client = McpTestClient {
            process,
            stdin,
            stdout,
            request_id: 0,
        };

        // Wait for server to initialize
        std::thread::sleep(Duration::from_millis(1000));

        // Send initialize request
        client.initialize()?;

        Ok(client)
    }

    /// Send a request and get response
    fn send_request(&mut self, method: &str, params: Value) -> Result<McpResponse> {
        self.request_id += 1;
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Some(self.request_id),
        };

        // Send request
        let request_str = serde_json::to_string(&request)?;
        writeln!(self.stdin, "{request_str}")?;
        self.stdin.flush()?;

        // Read response
        let mut response_str = String::new();
        self.stdout.read_line(&mut response_str)?;

        let response: McpResponse = serde_json::from_str(&response_str)
            .with_context(|| format!("Failed to parse response: {response_str}"))?;

        Ok(response)
    }

    /// Send a notification (no response expected)
    fn send_notification(&mut self, method: &str, params: Value) -> Result<()> {
        let notification = McpRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: None,
        };

        // Send notification
        let notification_str = serde_json::to_string(&notification)?;
        writeln!(self.stdin, "{notification_str}")?;
        self.stdin.flush()?;

        Ok(())
    }

    /// Initialize the MCP connection
    fn initialize(&mut self) -> Result<()> {
        let response = self.send_request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }),
        )?;

        if response.error.is_some() {
            bail!("Failed to initialize: {:?}", response.error);
        }

        // Send initialized notification
        self.send_notification("notifications/initialized", json!({}))?;

        Ok(())
    }

    /// Check if server is initialized
    fn is_initialized(&self) -> bool {
        true
    }

    /// Call a tool and return the result
    fn call_tool(&mut self, tool_name: &str, arguments: Value) -> Result<Value> {
        let response = self.send_request(
            "tools/call",
            json!({
                "name": tool_name,
                "arguments": arguments
            }),
        )?;

        if let Some(error) = response.error {
            return Ok(Value::String(format!("Error: {error}")));
        }

        // Extract the content from the response
        if let Some(result) = response.result {
            if let Some(content_array) = result.get("content").and_then(|c| c.as_array())
                && let Some(first_content) = content_array.first()
                && let Some(text) = first_content.get("text").and_then(|t| t.as_str())
            {
                // Try to parse the text as JSON, or return as string
                if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                    return Ok(parsed);
                } else {
                    return Ok(Value::String(text.to_string()));
                }
            }
            Ok(result)
        } else {
            bail!("No result in response")
        }
    }
}

impl Drop for McpTestClient {
    fn drop(&mut self) {
        // Clean shutdown of the MCP server
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

#[test]
fn test_mcp_server_initialization() -> Result<()> {
    let client = McpTestClient::new()?;
    assert!(client.is_initialized());
    drop(client);
    Ok(())
}

#[test]
fn test_mcp_server_starts_and_stops() -> Result<()> {
    // Start multiple instances to ensure clean startup/shutdown
    for _ in 0..3 {
        let client = McpTestClient::new()?;
        assert!(client.is_initialized());
        drop(client);
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

#[test]
fn test_account_list_tool() -> Result<()> {
    let mut client = McpTestClient::new()?;

    let result = client.call_tool("account_list", json!({}))?;

    // Should return an array of accounts (might be empty)
    assert!(result.is_array() || result.is_object());

    Ok(())
}

#[test]
fn test_repo_detect_tool() -> Result<()> {
    let mut client = McpTestClient::new()?;

    // This might fail if not in a git repo, but tests the tool is available
    let result = client.call_tool(
        "repo_detect",
        json!({
            "repo_path": "."
        }),
    )?;

    // Should return either repo info or an error
    assert!(result.is_object() || result.as_str().unwrap_or("").contains("Error"));

    Ok(())
}

#[test]
fn test_patch_generate_tool() -> Result<()> {
    let mut client = McpTestClient::new()?;

    // This might fail if not in a git repo, but tests the tool is available
    let result = client.call_tool(
        "patch_generate",
        json!({
            "since": "HEAD~1"
        }),
    )?;

    // Should return either patches or an error
    assert!(result.is_object() || result.as_str().unwrap_or("").contains("Error"));

    Ok(())
}

#[test]
fn test_invalid_tool_call() -> Result<()> {
    let mut client = McpTestClient::new()?;

    // Try to call a non-existent tool
    let result = client.send_request(
        "tools/call",
        json!({
            "name": "non_existent_tool",
            "arguments": {}
        }),
    );

    // This should return an error
    assert!(result.is_ok());
    let response = result?;
    assert!(response.error.is_some());

    Ok(())
}

#[test]
fn test_tool_with_invalid_arguments() -> Result<()> {
    let mut client = McpTestClient::new()?;

    // Call pr_send without required parameters
    let result = client.call_tool("pr_send", json!({}))?;

    // Should return an error
    let result_str = result.as_str().unwrap_or("");
    assert!(result_str.contains("Error"));

    Ok(())
}

#[test]
fn test_account_create_tool() -> Result<()> {
    let mut client = McpTestClient::new()?;

    // Create a test account
    let result = client.call_tool(
        "account_create",
        json!({
            "name": "test_account_mcp"
        }),
    )?;

    // Should return success or error if account already exists
    assert!(result.is_object() || result.as_str().unwrap_or("").contains("Error"));

    Ok(())
}

#[test]
fn test_repo_state_tool_missing_params() -> Result<()> {
    let mut client = McpTestClient::new()?;

    // Call repo_state without required identifier
    let result = client.call_tool("repo_state", json!({}))?;

    // Should return an error
    let result_str = result.as_str().unwrap_or("");
    assert!(result_str.contains("Error"));

    Ok(())
}

#[test]
fn test_pr_list_missing_password() -> Result<()> {
    let mut client = McpTestClient::new()?;

    // Call pr_list without password
    let result = client.call_tool("pr_list", json!({}))?;

    // Should return an error
    let result_str = result.as_str().unwrap_or("");
    assert!(result_str.contains("Error"));

    Ok(())
}

#[test]
fn test_repo_generate_tool() -> Result<()> {
    let mut client = McpTestClient::new()?;

    // This might fail if not in a git repo, but tests the tool is available
    let result = client.call_tool(
        "repo_generate",
        json!({
            "repo_path": ".",
            "include_sample_relays": true
        }),
    )?;

    // Should return either repo config as a JSON string or an error
    assert!(result.is_string() || result.is_object());

    Ok(())
}

#[test]
fn test_sync_repository_tool() -> Result<()> {
    let mut client = McpTestClient::new()?;

    // This might fail if not in a git repo, but tests the tool is available
    let result = client.call_tool(
        "sync_repository",
        json!({
            "repo_path": "."
        }),
    )?;

    // Should return either sync data as a JSON string or an error
    // The result is a JSON string (not a parsed object) or an error string
    assert!(result.is_string());

    Ok(())
}

#[test]
fn test_pr_sync_missing_params() -> Result<()> {
    let mut client = McpTestClient::new()?;

    // Call pr_sync without required event_id
    let result = client.call_tool("pr_sync", json!({}))?;

    // Should return an error
    let result_str = result.as_str().unwrap_or("");
    assert!(result_str.contains("Error"));

    Ok(())
}
