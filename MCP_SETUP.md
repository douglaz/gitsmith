# GitSmith MCP Server Setup

This guide explains how to configure and use the gitsmith MCP (Model Context Protocol) server with Claude Code.

## What is MCP?

MCP (Model Context Protocol) is Anthropic's standard for exposing tools and commands to AI assistants. The gitsmith MCP server allows Claude to directly interact with Git/Nostr repository operations.

## Available Tools

The gitsmith MCP server exposes 12 tools organized into four categories:

### Account Management Tools
- `account_create` - Create a new Nostr account
- `account_import` - Import an existing Nostr account
- `account_login` - Login to a gitsmith account
- `account_list` - List all gitsmith accounts
- `account_export` - Export account backup

### Repository Tools
- `repo_init` - Initialize a repository on Nostr
- `repo_detect` - Auto-detect repository from git
- `repo_state` - Get repository state

### Pull Request Tools
- `pr_send` - Send a pull request to Nostr
- `pr_list` - List pull requests

### Patch Tools
- `patch_send` - Send patches to Nostr
- `patch_generate` - Generate patches from commits

## Setup Instructions

### 1. Install gitsmith

First, install gitsmith to your system:

**Option A: Install to ~/bin (Recommended)**
```bash
# Build and copy to ~/bin
cd /path/to/gitsmith
cargo build --release
cp target/release/gitsmith ~/bin/
```

**Option B: Install via cargo**
```bash
cargo install --path gitsmith
```

### 2. Configure MCP with Claude Code CLI

Use the Claude Code CLI to add the gitsmith MCP server:

#### Global Setup (Recommended - Available in All Projects)

```bash
# Add gitsmith MCP server globally for all projects
claude mcp add gitsmith ~/bin/gitsmith mcp-server -s user -e RUST_LOG=info
```

This makes gitsmith tools available in every project without additional configuration.

#### Project-Specific Setup (For Team Sharing)

If you want to share the configuration with your team via version control:

```bash
# Add to project configuration
claude mcp add gitsmith gitsmith mcp-server -s project -e RUST_LOG=info
```

This creates/updates `.mcp.json` in your project root that can be committed to version control.

### 3. Verify MCP Server Connection

Check that the MCP server is properly connected:

```bash
# List all configured MCP servers and their status
claude mcp list
```

You should see:
```
Checking MCP server health...
gitsmith: ~/bin/gitsmith mcp-server - ✓ Connected
```

### 4. Manage MCP Servers

**View configuration:**
```bash
# Show all MCP servers across different scopes
claude mcp list
```

**Remove a server:**
```bash
# Remove from specific scope
claude mcp remove gitsmith -s user    # Remove from global config
claude mcp remove gitsmith -s project  # Remove from project config
claude mcp remove gitsmith -s local    # Remove from local config
```

**Update configuration:**
```bash
# Remove and re-add with new settings
claude mcp remove gitsmith -s user
claude mcp add gitsmith ~/bin/gitsmith mcp-server -s user -e RUST_LOG=debug -e GITSMITH_PASSWORD=mypass
```

## Usage Examples

Once configured, you can ask Claude to use gitsmith tools directly:

### Account Management
```
"Create a new gitsmith account named 'dev'"
"List all my gitsmith accounts"
"Export my account 'dev' with password 'secret'"
```

### Repository Operations
```
"Detect repository information from the current directory"
"Initialize this repository on Nostr with identifier 'my-project'"
"Get the state of repository 'my-project'"
```

### Pull Requests
```
"Send a PR with title 'Fix bug' and description 'Fixes issue #123'"
"List all pull requests for this repository"
```

### Patches
```
"Generate patches from the last 3 commits"
"Send patches from HEAD~2"
```

## Configuration Options

### Environment Variables

You can pass environment variables to the MCP server:

```json
{
  "mcpServers": {
    "gitsmith": {
      "command": "gitsmith",
      "args": ["mcp-server"],
      "env": {
        "RUST_LOG": "debug",  // Set log level
        "GITSMITH_PASSWORD": "default_password"  // Default password for operations
      }
    }
  }
}
```

### Transport Options

Currently, the MCP server supports stdio transport (default). SSE transport support is planned for future releases.

## Troubleshooting

### Server Not Connecting

1. **Check binary exists and is executable:**
   ```bash
   ls -la ~/bin/gitsmith
   # Should show executable permissions (x)
   ```

2. **Test the MCP server directly:**
   ```bash
   # This should start without errors
   ~/bin/gitsmith mcp-server
   # Press Ctrl+C to stop
   ```

3. **Check MCP connection status:**
   ```bash
   claude mcp list
   ```

### Common Issues

**"Failed to connect" error:**
- Make sure you're using the full path to the binary (e.g., `~/bin/gitsmith` or `/home/user/bin/gitsmith`)
- The relative path `gitsmith` only works if it's in your PATH

**Multiple scope conflicts:**
- If you have the same server in multiple scopes, remove duplicates:
  ```bash
  claude mcp remove gitsmith -s project
  claude mcp remove gitsmith -s local
  # Keep only the user scope for global access
  ```

**Binary not found:**
- Ensure gitsmith is built: `cargo build --release`
- Copy to ~/bin: `cp target/release/gitsmith ~/bin/`
- Make executable: `chmod +x ~/bin/gitsmith`

**Account password issues:**
- Many operations require a password to decrypt account keys
- You can set a default password via environment variable: `GITSMITH_PASSWORD`
- Or provide it in each tool call

## Security Considerations

- The MCP server runs with your local user permissions
- Account passwords are used to encrypt/decrypt Nostr private keys
- Never commit passwords or private keys to version control
- Use environment variables for sensitive configuration
- Consider using separate accounts for different projects

## Development

To run the MCP server in development mode:

```bash
RUST_LOG=debug cargo run -- mcp-server
```

This will start the server with stdio transport and detailed logging.

## Testing

Run the integration tests to verify MCP functionality:

```bash
cargo test mcp_integration_test
```

## Future Enhancements

Planned improvements for the MCP server:
- SSE transport support for network access
- WebSocket transport option
- PR sync functionality
- Issue management tools
- Enhanced error reporting with recovery suggestions
- Batch operations for multiple repositories