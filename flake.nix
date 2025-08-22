{
  description = "GitSmith - Forge your git repositories on Nostr";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "x86_64-unknown-linux-musl" ];
        };
      in
      {
        # Default package: static musl build
        packages.default = let
          rustPlatformMusl = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };
        in rustPlatformMusl.buildRustPackage {
          pname = "gitsmith";
          version = "0.1.0";
          src = ./.;
          
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          
          nativeBuildInputs = with pkgs; [
            pkg-config
            rustToolchain
            pkgsStatic.stdenv.cc
          ];
          
          buildInputs = with pkgs.pkgsStatic; [
            openssl
          ];
          
          # Environment variables for static OpenSSL
          OPENSSL_STATIC = "1";
          OPENSSL_LIB_DIR = "${pkgs.pkgsStatic.openssl.out}/lib";
          OPENSSL_INCLUDE_DIR = "${pkgs.pkgsStatic.openssl.dev}/include";
          PKG_CONFIG_PATH = "${pkgs.pkgsStatic.openssl.dev}/lib/pkgconfig";
          
          # Force cargo to use the musl target
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "${pkgs.pkgsStatic.stdenv.cc}/bin/${pkgs.pkgsStatic.stdenv.cc.targetPrefix}cc";
          CC_x86_64_unknown_linux_musl = "${pkgs.pkgsStatic.stdenv.cc}/bin/${pkgs.pkgsStatic.stdenv.cc.targetPrefix}cc";
          CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static -C link-arg=-static";
          
          # Override buildPhase to use the correct target
          buildPhase = ''
            runHook preBuild
            
            echo "Building GitSmith with musl target for static binary..."
            cargo build \
              --release \
              --target x86_64-unknown-linux-musl \
              --offline \
              -j $NIX_BUILD_CORES
            
            runHook postBuild
          '';
          
          installPhase = ''
            runHook preInstall
            
            mkdir -p $out/bin
            cp target/x86_64-unknown-linux-musl/release/gitsmith $out/bin/
            
            runHook postInstall
          '';
          
          # Ensure static linking
          doCheck = false; # Tests don't work well with static linking
          
          # Verify the binary is statically linked
          postInstall = ''
            echo "Checking if binary is statically linked..."
            file $out/bin/gitsmith
            # Strip the binary to reduce size
            ${pkgs.pkgsStatic.stdenv.cc.targetPrefix}strip $out/bin/gitsmith
            echo "Binary size after stripping:"
            ls -lh $out/bin/gitsmith
          '';
          
          meta = with pkgs.lib; {
            description = "Forge your git repositories on Nostr";
            homepage = "https://github.com/douglaz/gitsmith";
            license = licenses.mit;
            maintainers = [ ];
            platforms = [ "x86_64-linux" ];
          };
        };
        
        # Development shell
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            rust-analyzer
            pkg-config
            openssl
            git
            jq
            
            # For libgit2
            libgit2
            libssh2
            zlib
            
            # Development tools
            cargo-edit
            cargo-watch
            cargo-outdated
            cargo-audit
            cargo-deny
            cargo-expand
            cargo-udeps
            
            # For static builds
            pkgsStatic.stdenv.cc
            pkgsStatic.openssl
          ];
          
          RUST_BACKTRACE = 1;
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          
          shellHook = ''
            # Configure git hooks if in a git repository
            if [ -d .git ] && [ -d .githooks ]; then
              current_hooks_path=$(git config core.hooksPath || echo "")
              if [ "$current_hooks_path" != ".githooks" ]; then
                echo "📎 Setting up git hooks..."
                git config core.hooksPath .githooks
                echo "   Git hooks configured to use .githooks/"
                echo "   Pre-push checks will run automatically"
                echo ""
                echo "To disable: git config --unset core.hooksPath"
                echo ""
              fi
            fi
            
            echo "🔨 GitSmith development environment"
            echo "Rust version: $(rustc --version)"
            echo ""
            echo "Available commands:"
            echo "  cargo build           - Build the project"
            echo "  cargo test            - Run tests"
            echo "  cargo clippy          - Run linter"
            echo "  cargo fmt             - Format code"
            echo "  nix build             - Build static musl binary"
            echo ""
            echo "Git hooks:"
            echo "  .githooks/pre-push    - Runs fmt, clippy, and tests before push"
            echo ""
          '';
        };
      });
}