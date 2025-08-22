# GitSmith

🔨 **Forge your git repositories on Nostr** - A non-interactive CLI tool for publishing git repositories to the Nostr protocol, implementing NIP-34.

## Quick Start

### Option 1: Using the Auto-installer Script

```bash
# Download and run GitSmith directly
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

GitSmith uses Nix for reproducible development environments and builds:

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

GitSmith uses a clean workspace structure:

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

## NIP-34 Compatibility

Implements NIP-34 (git stuff) specification:
- Kind 30617: Repository announcements
- Kind 30618: Git state updates
- Kind 1617: Patches (planned)

## Key Features

- **Zero-dependency installation** via `gitsmith.sh`
- **Static binary builds** with Nix and musl
- **Non-interactive CLI** with environment variable support
- **Clean architecture** separating core logic from CLI
- **Automatic updates** with version checking
- **Git hooks** for code quality
- **CI/CD** with GitHub Actions

## License

MIT