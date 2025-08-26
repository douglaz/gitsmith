# gitsmith

🔨 **Forge your git repositories on Nostr** - A Git-compatible interface for decentralized code collaboration using the Nostr protocol.

gitsmith brings familiar Git workflows to Nostr, enabling developers to create pull requests, submit patches, and collaborate on code without centralized platforms. It implements ngit compatibility and NIP-34 specifications for seamless integration with the Nostr ecosystem.

## Quick Start

### Option 1: Using the Auto-installer Script

```bash
# Download and run gitsmith directly
curl -sSL https://raw.githubusercontent.com/douglaz/gitsmith/master/gitsmith.sh | bash -s -- --help

# Or download the script for repeated use
curl -sSL https://raw.githubusercontent.com/douglaz/gitsmith/master/gitsmith.sh -o gitsmith.sh
chmod +x gitsmith.sh
./gitsmith.sh --help
```

The `gitsmith.sh` script automatically:
- Downloads the latest release for your platform
- Installs it to `~/.local/bin`
- Checks for updates daily
- Falls back to building from source if no release is available

### Option 2: Install with Nix

```bash
# Run directly from GitHub (one-time use)
nix run github:douglaz/gitsmith -- --help

# Install to your system
nix profile install github:douglaz/gitsmith
gitsmith --help
```

### Option 3: Build from Source with Nix (Recommended for Development)

```bash
# Clone the repository
git clone https://github.com/douglaz/gitsmith.git
cd gitsmith

# Enter development environment (includes all dependencies)
nix develop

# Build and run
cargo build --release
./target/release/gitsmith --help

# Or build static musl binary
nix build
./result/bin/gitsmith --help
```

## Development with Nix

gitsmith uses Nix for reproducible development environments and builds:

```bash
# Enter development shell with all tools
nix develop

# The shell provides:
# - Rust toolchain with musl target
# - OpenSSL static libraries
# - Git hooks auto-configuration
# - Development tools (cargo-watch, cargo-audit, etc.)

# Available commands in nix develop:
cargo build           # Build the project
cargo test            # Run tests
cargo clippy          # Run linter
cargo fmt             # Format code
nix build             # Build static musl binary

# Git hooks are automatically configured on entry
# Pre-push hook runs: fmt check, clippy, and tests
```

### Building Static Binaries

```bash
# Build static musl binary (portable, works anywhere)
nix build

# The result is a fully static binary
file ./result/bin/gitsmith
# gitsmith: ELF 64-bit LSB executable, statically linked

# Copy to any Linux system
cp ./result/bin/gitsmith /usr/local/bin/
```

## Installation Methods Comparison

| Method | Use Case | Updates | Dependencies |
|--------|----------|---------|--------------|
| `gitsmith.sh` | End users, easy updates | Automatic daily | None (downloads binary) |
| `nix run` | Try without installing | Manual | Nix |
| `nix profile install` | Nix users | Manual via nix | Nix |
| `nix develop` + `cargo` | Development | Manual | Nix |
| `nix build` | Static binary distribution | Manual | Nix (build-time only) |

## Developer Workflow

### Complete Development Workflow with Git and Nostr

gitsmith enables a complete development workflow similar to GitHub/GitLab but on the decentralized Nostr protocol. Here's how to use it for real development:

#### 1. Setting Up Your Account

First, create and configure your Nostr account for development:

```bash
# Create a new Nostr account for development
gitsmith account create --name "Alice Developer"

# Or import existing Nostr private key
gitsmith account import --nsec "nsec1..." --name "My Dev Account"

# Login to your account (sets it as active)
gitsmith account login
# Enter password when prompted

# List all accounts
gitsmith account list

# Export your account (backup)
gitsmith account export --name "My Dev Account" > my-account.json
```

#### 2. Initialize a Repository on Nostr

Make your git repository available on Nostr:

```bash
# Navigate to your git repository
cd my-project

# Initialize it on Nostr (auto-detects git config)
gitsmith init
# This will:
# - Detect repository name and description from git
# - Generate a unique identifier
# - Configure default relays
# - Create a repository announcement on Nostr

# Or manually specify details
gitsmith init \
    --name "My Awesome Project" \
    --description "A revolutionary new tool" \
    --identifier "my-awesome-project" \
    --relay wss://relay.damus.io \
    --relay wss://nos.lol \
    --relay wss://relay.nostr.band
```

#### 3. Creating Pull Requests

Submit code changes as pull requests to Nostr:

```bash
# Make your changes
git checkout -b feature/new-feature
echo "// New feature" >> src/main.rs
git add -A
git commit -m "feat: add new feature"

# Send a pull request (defaults to HEAD~1)
gitsmith send pr \
    --title "Add new feature" \
    --description "This PR adds an amazing new feature"

# Send multiple commits as a PR
git commit -m "feat: part 1"
git commit -m "feat: part 2"
gitsmith send pr HEAD~2 \
    --title "Multi-commit feature" \
    --description "This PR contains multiple improvements"

# Send as a patch series (individual patches)
gitsmith send patch HEAD~3 \
    --title "Refactoring patch series"
```

#### 4. Reviewing and Managing Pull Requests

View and interact with pull requests:

```bash
# List all PRs for current repository
gitsmith list prs

# List PRs with specific status
gitsmith list prs --status open
gitsmith list prs --status merged
gitsmith list prs --status closed

# Show detailed PR information
gitsmith list prs --format long

# List PRs for a specific repository
gitsmith list prs --identifier "other-project"

# Sync and fetch updates for a specific PR
gitsmith sync pr <event-id>
```

#### 5. Working with Patches

For smaller changes or traditional patch workflow:

```bash
# Send a single patch
git commit -m "fix: resolve bug"
gitsmith send patch

# List all patches
gitsmith list patches

# Apply a patch from Nostr
gitsmith sync patch <event-id>
# This fetches the patch and shows how to apply it with git am
```

#### 6. Collaboration Workflow Example

Here's a complete example of collaborating on a project:

```bash
# Developer A: Initialize project
cd cool-project
gitsmith init --name "Cool Project"

# Developer A: Push initial code
git add .
git commit -m "Initial commit"
gitsmith send pr HEAD~1 --title "Initial implementation"

# Developer B: Clone and contribute
git clone https://github.com/user/cool-project
cd cool-project
gitsmith init  # Connect to same Nostr repository

# Developer B: Create feature
git checkout -b feature/awesome
echo "// Awesome code" >> awesome.rs
git commit -am "feat: add awesome feature"
gitsmith send pr --title "Add awesome feature" \
    --description "This adds the awesome feature we discussed"

# Developer A: Review PRs
gitsmith list prs
gitsmith sync pr <event-id-of-awesome-feature>

# Developer A: Merge locally and announce
git merge feature/awesome
git push origin main
gitsmith send patch HEAD~1 --title "Merged: awesome feature"
```

#### 7. Multiple Relay Strategy

Use multiple relays for redundancy and reach:

```bash
# Configure project with multiple relays
gitsmith init \
    --relay wss://relay.damus.io \
    --relay wss://relay.nostr.band \
    --relay wss://nos.lol \
    --relay wss://relay.snort.social \
    --relay wss://nostr.wine

# PRs and patches are sent to ALL configured relays
gitsmith send pr --title "Important fix"
# Sends to all 5 relays for maximum visibility
```

## Common Use Cases

### Starting a New Open Source Project

```bash
# Create your project
mkdir my-new-tool && cd my-new-tool
git init
echo "# My New Tool" > README.md
git add . && git commit -m "Initial commit"

# Set up gitsmith account
gitsmith account create --name "Your Name"
gitsmith account login

# Announce to Nostr
gitsmith init --name "My New Tool" \
    --description "A tool that solves X problem" \
    --relay wss://relay.damus.io \
    --relay wss://nos.lol

# Start accepting contributions!
```

### Contributing to Existing Projects

```bash
# Find project on Nostr (using a Nostr client)
# Clone the repository using provided git URL
git clone https://github.com/someone/project
cd project

# Connect to Nostr repository
gitsmith init  # Auto-detects from .git/config

# Make your contribution
git checkout -b fix/bug-123
# ... make changes ...
git commit -m "fix: resolve issue #123"

# Submit PR via Nostr
gitsmith send pr --title "Fix for issue #123" \
    --description "This fixes the bug where..."
```

### Maintaining a Mirror on Nostr

Keep your GitHub/GitLab project also available on Nostr:

```bash
# In your existing project
cd my-github-project

# Set up dual presence
gitsmith init --name "My Project (Nostr Mirror)" \
    --clone-url "https://github.com/user/project"

# After pushing to GitHub, announce on Nostr
git push origin main
gitsmith send patch HEAD~1 --title "Latest updates"
```

### Code Review Workflow

```bash
# Reviewer: List PRs needing review
gitsmith list prs --status open

# Fetch PR to review
gitsmith sync pr <event-id>
# Creates a branch with the PR changes

# Review the code
git diff main...pr-branch
# Test changes locally
cargo test  # or npm test, etc.

# Provide feedback (using Nostr client)
# Reply to the PR event with comments
```

## Best Practices

### 1. Account Security

- **Backup your keys**: Always export and securely store your account
  ```bash
  gitsmith account export --name "Dev Account" > ~/secure-backup/account.json
  ```
- **Use strong passwords**: Account keys are encrypted with your password
- **One account per identity**: Use different accounts for personal/work

### 2. Relay Selection

- **Use 3-5 relays**: Balance between redundancy and performance
- **Mix relay types**: Combine large public and smaller community relays
- **Verify relay uptime**: Test relays before adding to critical projects

### 3. Repository Management

- **Clear identifiers**: Use descriptive, unique identifiers
  ```bash
  # Good
  gitsmith init --identifier "bitcoin-wallet-lib"
  
  # Bad
  gitsmith init --identifier "wallet"
  ```
- **Complete descriptions**: Help others understand your project
- **Include clone URLs**: Make it easy for others to get the code

### 4. Pull Request Etiquette

- **Descriptive titles**: Summarize the change clearly
- **Detailed descriptions**: Explain why, not just what
- **Atomic commits**: Each commit should be a logical unit
- **Reference issues**: Link to related discussions

### 5. Performance Tips

- **Batch operations**: Send multiple patches together
- **Local-first**: Work locally, sync to Nostr when ready
- **Relay limits**: Be aware some relays have rate limits

## Troubleshooting

### Common Issues and Solutions

#### "No active account found"
```bash
# Solution: Login to your account
gitsmith account list  # See available accounts
gitsmith account login
```

#### "Failed to connect to relay"
```bash
# Solution: Try different relays
gitsmith init --relay wss://different-relay.com
```

#### "Repository not found on Nostr"
```bash
# Solution: Ensure repository is announced
gitsmith init  # Re-announce if needed
```

#### "PR not showing up"
```bash
# Solution: Check relay propagation
gitsmith list prs  # May take a moment to propagate
# Try adding more relays for better reach
```

## Quick Command Reference

### Account Management
```bash
gitsmith account create --name <name>           # Create new account
gitsmith account import --nsec <key>            # Import existing key  
gitsmith account login                          # Set active account
gitsmith account logout                         # Clear active account
gitsmith account list                           # Show all accounts
gitsmith account export --name <name>           # Export account backup
```

### Repository Operations  
```bash
gitsmith init                                   # Initialize repo on Nostr
gitsmith init --relay <url>                    # Specify custom relays
gitsmith state --identifier <id>                # View repository state
```

### Sending Changes
```bash
gitsmith send pr                               # Send PR (HEAD~1 default)
gitsmith send pr HEAD~3                        # Send last 3 commits as PR
gitsmith send patch                            # Send single patch
gitsmith send patch HEAD~2                     # Send last 2 commits as patches
```

### Viewing & Syncing
```bash
gitsmith list prs                              # List all PRs
gitsmith list prs --status open                # Filter by status
gitsmith list patches                          # List all patches  
gitsmith sync pr <event-id>                    # Fetch PR locally
gitsmith sync patch <event-id>                 # Fetch patch locally
```

## Usage

### Publishing a Repository to Nostr

```bash
# Quick publish with gitsmith.sh
./gitsmith.sh init \
    --identifier "my-project" \
    --name "My Project" \
    --description "A great project" \
    --clone-url "https://github.com/user/repo.git" \
    --relay "wss://relay.damus.io" \
    --relay "wss://nos.lol" \
    --nsec "your_private_key_hex" \
    --repo-path .

# Or with environment variables
export GITSMITH_IDENTIFIER="my-project"
export GITSMITH_NAME="My Project"
export GITSMITH_DESCRIPTION="A great project"
export GITSMITH_CLONE_URLS="https://github.com/user/repo.git"
export GITSMITH_RELAYS="wss://relay.damus.io,wss://nos.lol"
export NOSTR_PRIVATE_KEY="your_private_key_hex"

./gitsmith.sh init
```

### Other Commands

```bash
# Generate repository configuration from existing git repo
gitsmith generate --repo-path . --include-sample-relays -o repo.json

# View current git state
gitsmith state --identifier "my-project" --output json

# Get help
gitsmith --help
gitsmith init --help
```

## How gitsmith.sh Works

The `gitsmith.sh` script provides zero-dependency installation and updates:

1. **Auto-detection**: Checks for local development builds first
2. **Version management**: Tracks installed version, checks for updates daily
3. **Platform detection**: Downloads appropriate binary for your system
4. **Fallback building**: If no release exists, attempts to build from source
5. **Path management**: Installs to `~/.local/bin` (add to PATH if needed)

```bash
# Manual update check
./gitsmith.sh --version  # Triggers update check if needed

# Force reinstall
rm ~/.local/bin/.gitsmith.version
./gitsmith.sh --help  # Will download latest
```

## Architecture

gitsmith uses a clean workspace structure:

- **`gitsmith-core`**: Core library with all Nostr/git logic
- **`gitsmith`**: CLI application using the core library
- **`flake.nix`**: Nix build configuration for reproducible builds
- **`.githooks/`**: Pre-push checks for code quality
- **`.github/workflows/`**: CI/CD for testing and releases

## Library Usage

The `gitsmith-core` crate can be used as a dependency:

```toml
[dependencies]
gitsmith-core = { git = "https://github.com/douglaz/gitsmith" }
```

```rust
use gitsmith_core::{
    announce_repository, detect_from_git,
    RepoAnnouncement, PublishConfig
};

#[tokio::main]
async fn main() -> Result<()> {
    // Auto-detect from git repository
    let announcement = detect_from_git(".")?;
    
    let config = PublishConfig {
        timeout_secs: 30,
        wait_for_send: true,
    };
    
    let result = announce_repository(
        announcement,
        "private_key_hex",
        config
    ).await?;
    
    println!("Published to Nostr: {}", result.nostr_url);
    Ok(())
}
```

## Contributing

```bash
# Fork and clone
git clone https://github.com/yourusername/gitsmith.git
cd gitsmith

# Enter development environment
nix develop

# Make changes and test
cargo test
cargo clippy
cargo fmt

# Git hooks run automatically on push
git push  # Runs fmt check, clippy, and tests
```

## Why gitsmith?

### Advantages Over Traditional Platforms

- **Decentralized**: No single point of failure or control
- **Censorship-resistant**: Your code can't be taken down by platform decisions
- **Identity ownership**: You control your developer identity via Nostr keys
- **Platform agnostic**: Works alongside GitHub, GitLab, or any git hosting
- **Privacy-focused**: Choose which relays see your activity
- **No vendor lock-in**: Your data lives on the Nostr protocol, not a company's servers

### When to Use gitsmith

- **Open source projects** seeking true decentralization
- **Backup collaboration** channel for critical projects  
- **Cross-platform projects** needing neutral ground
- **Privacy-sensitive development** requiring control over data distribution
- **Experimental/controversial projects** that might face platform restrictions
- **Learning projects** to understand decentralized development

## Key Features

### Core Functionality
- **Git-compatible workflow**: Familiar PR and patch paradigms
- **ngit compatibility**: Works with other Nostr git implementations
- **Multi-relay support**: Redundancy and censorship resistance
- **Account management**: Multiple developer identities
- **Comprehensive CLI**: Full-featured command-line interface

### Technical Features  
- **Zero-dependency installation** via `gitsmith.sh`
- **Static binary builds** with Nix and musl
- **Non-interactive CLI** with environment variable support
- **Clean architecture** separating core logic from CLI
- **Automatic updates** with version checking
- **Git hooks** for code quality
- **CI/CD** with GitHub Actions

### Protocol Support
- **NIP-34**: Git stuff specification
- **Kind 30617**: Repository announcements
- **Kind 30618**: Git state updates  
- **Kind 1617**: Patches and pull requests

## License

MIT