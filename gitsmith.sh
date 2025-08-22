#!/usr/bin/env bash
set -euo pipefail

# Configuration
REPO="douglaz/gitsmith"
BINARY_NAME="gitsmith"
INSTALL_DIR="${HOME}/.local/bin"
INSTALLED_VERSION_FILE="${INSTALL_DIR}/.${BINARY_NAME}.version"

# Determine script directory (only available when run as a file, not via stdin)
if [[ -n "${BASH_SOURCE[0]:-}" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
else
    SCRIPT_DIR=""
fi

# Platform detection
detect_platform() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)
    
    case "$os" in
        linux)
            case "$arch" in
                x86_64) echo "linux-x86_64" ;;
                *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        darwin)
            case "$arch" in
                x86_64) echo "macos-x86_64" ;;
                arm64) echo "macos-aarch64" ;;
                *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        mingw*|cygwin*|msys*)
            echo "windows-x86_64"
            ;;
        *) echo "Unsupported OS: $os" >&2; exit 1 ;;
    esac
}

# Get latest release version from GitHub
get_latest_version() {
    # For gitsmith, we use a fixed "latest-master" tag for continuous deployment
    # Check if the release exists
    local release_info=$(curl -s "https://api.github.com/repos/$REPO/releases/tags/latest-master" 2>/dev/null)
    
    # Check if we got a valid response (should have an "id" field if the release exists)
    if echo "$release_info" | grep -q '"id"'; then
        echo "latest-master"
    else
        # No release found
        echo ""
    fi
}

# Get currently installed version (stores commit SHA for latest-master)
get_installed_version() {
    if [[ -f "$INSTALLED_VERSION_FILE" ]]; then
        cat "$INSTALLED_VERSION_FILE"
    else
        echo "none"
    fi
}

# Get the commit SHA for the latest-master release
get_latest_commit() {
    local release_info=$(curl -s "https://api.github.com/repos/$REPO/releases/tags/latest-master" 2>/dev/null)
    
    # Extract the target_commitish (commit SHA) from the release
    if command -v jq >/dev/null 2>&1; then
        echo "$release_info" | jq -r '.target_commitish // empty' 2>/dev/null
    else
        echo "$release_info" | grep -o '"target_commitish":"[^"]*"' | cut -d'"' -f4
    fi
}

# Download and install binary
install_binary() {
    local version="$1"
    local platform="$2"
    
    echo "Downloading ${BINARY_NAME} ${version} for ${platform}..." >&2
    
    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"
    
    # Determine archive type and construct URL
    local archive_ext="tar.gz"
    local binary_name="${BINARY_NAME}"
    if [[ "$platform" == windows-* ]]; then
        archive_ext="zip"
        binary_name="${BINARY_NAME}.exe"
    fi
    
    local url="https://github.com/${REPO}/releases/download/${version}/${BINARY_NAME}-${platform}.${archive_ext}"
    
    # Download to temporary file
    local temp_file=$(mktemp)
    if ! curl -sL -o "$temp_file" "$url"; then
        rm -f "$temp_file"
        echo "Failed to download ${BINARY_NAME}" >&2
        exit 1
    fi
    
    # Extract the archive
    local temp_dir=$(mktemp -d)
    if [[ "$archive_ext" == "zip" ]]; then
        # Use unzip for Windows archives
        if command -v unzip >/dev/null 2>&1; then
            unzip -q "$temp_file" -d "$temp_dir"
        else
            echo "Error: unzip is required for Windows binaries" >&2
            rm -f "$temp_file"
            rm -rf "$temp_dir"
            exit 1
        fi
    else
        tar -xzf "$temp_file" -C "$temp_dir"
    fi
    rm -f "$temp_file"
    
    # Find and move the binary (archive contains platform dir with binary inside)
    local binary_path="${temp_dir}/${platform}/${binary_name}"
    
    if [[ ! -f "$binary_path" ]]; then
        echo "Error: Binary not found in archive" >&2
        rm -rf "$temp_dir"
        exit 1
    fi
    
    # Make executable and move to install directory
    chmod +x "$binary_path"
    mv "$binary_path" "${INSTALL_DIR}/${BINARY_NAME}"
    rm -rf "$temp_dir"
    
    # Note: version recording is now done in main() after checking commit SHA
    
    echo "${BINARY_NAME} ${version} installed successfully" >&2
}

# Check for updates periodically (once per day)
should_check_update() {
    local check_file="${INSTALL_DIR}/.${BINARY_NAME}.last_check"
    
    # Always check if binary doesn't exist
    if [[ ! -f "${INSTALL_DIR}/${BINARY_NAME}" ]]; then
        return 0
    fi
    
    # Check if we've checked recently
    if [[ -f "$check_file" ]]; then
        local last_check=$(stat -c %Y "$check_file" 2>/dev/null || stat -f %m "$check_file" 2>/dev/null || echo 0)
        local current_time=$(date +%s)
        local day_in_seconds=86400
        
        if (( current_time - last_check < day_in_seconds )); then
            return 1
        fi
    fi
    
    # Mark that we're checking now
    touch "$check_file"
    return 0
}

# Main logic
main() {
    # First, check if we're in the repository and can run locally
    if [[ -n "$SCRIPT_DIR" && -d "${SCRIPT_DIR}/.git" ]]; then
        # Check if we have a local build
        local local_binary="${SCRIPT_DIR}/target/release/${BINARY_NAME}"
        if [[ ! -f "$local_binary" ]]; then
            local_binary="${SCRIPT_DIR}/target/x86_64-unknown-linux-musl/release/${BINARY_NAME}"
        fi
        if [[ ! -f "$local_binary" ]]; then
            local_binary="${SCRIPT_DIR}/target/debug/${BINARY_NAME}"
        fi
        
        if [[ -f "$local_binary" ]]; then
            # Use local build directly
            exec "$local_binary" "$@"
        fi
    fi
    
    local platform=$(detect_platform)
    
    # Check if we should look for updates
    if should_check_update; then
        local latest_version=$(get_latest_version)
        
        if [[ -n "$latest_version" ]]; then
            local latest_commit=$(get_latest_commit)
            local installed_version=$(get_installed_version)
            
            # Compare commit SHA to see if update is needed
            if [[ -n "$latest_commit" && "$latest_commit" != "$installed_version" ]]; then
                echo "New version available (commit: ${latest_commit:0:7})" >&2
                install_binary "$latest_version" "$platform"
                # Store the commit SHA as the installed version
                echo "$latest_commit" > "$INSTALLED_VERSION_FILE"
            fi
        fi
    fi
    
    # Check if binary exists in install dir
    if [[ -f "${INSTALL_DIR}/${BINARY_NAME}" ]]; then
        exec "${INSTALL_DIR}/${BINARY_NAME}" "$@"
    fi
    
    # No installed binary - try to download latest release
    local latest_version=$(get_latest_version)
    if [[ -n "$latest_version" ]]; then
        echo "Installing ${BINARY_NAME} ${latest_version}..." >&2
        install_binary "$latest_version" "$platform"
        
        # Store the commit SHA as installed version
        local latest_commit=$(get_latest_commit)
        if [[ -n "$latest_commit" ]]; then
            echo "$latest_commit" > "$INSTALLED_VERSION_FILE"
        fi
        
        # After successful install, run the binary
        if [[ -f "${INSTALL_DIR}/${BINARY_NAME}" ]]; then
            exec "${INSTALL_DIR}/${BINARY_NAME}" "$@"
        fi
    else
        # No releases available - check if we have a local development build
        echo "No releases available. Trying to build locally..." >&2
        
        if [[ -n "$SCRIPT_DIR" && -d "${SCRIPT_DIR}/.git" ]]; then
            cd "$SCRIPT_DIR"
            
            # Try to build with cargo if available
            if command -v cargo >/dev/null 2>&1; then
                echo "Building ${BINARY_NAME} from source..." >&2
                cargo build --release 2>/dev/null || cargo build
                
                # Try to find the built binary
                local local_binary="${SCRIPT_DIR}/target/release/${BINARY_NAME}"
                if [[ ! -f "$local_binary" ]]; then
                    local_binary="${SCRIPT_DIR}/target/debug/${BINARY_NAME}"
                fi
                
                if [[ -f "$local_binary" ]]; then
                    exec "$local_binary" "$@"
                fi
            fi
        fi
        
        echo "Error: Unable to find or build ${BINARY_NAME}." >&2
        echo "Please check https://github.com/${REPO} or build from source." >&2
        exit 1
    fi
}

# Run main function
main "$@"