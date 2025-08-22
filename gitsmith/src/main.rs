use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use gitsmith_core::{
    PublishConfig, RepoAnnouncement, announce_repository, detect_from_git, get_git_state,
    update_git_config,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gitsmith")]
#[command(about = "Publish git repositories to Nostr")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize and announce a repository on Nostr
    Init {
        /// Repository identifier (unique, no spaces)
        #[arg(long, env = "NOSTR_GIT_IDENTIFIER")]
        identifier: String,

        /// Repository name
        #[arg(long, env = "NOSTR_GIT_NAME")]
        name: String,

        /// Repository description
        #[arg(long, env = "NOSTR_GIT_DESCRIPTION")]
        description: String,

        /// Clone URLs (can be specified multiple times)
        #[arg(
            long = "clone-url",
            env = "NOSTR_GIT_CLONE_URLS",
            value_delimiter = ','
        )]
        clone_urls: Vec<String>,

        /// Nostr relays (can be specified multiple times)
        #[arg(long = "relay", env = "NOSTR_GIT_RELAYS", value_delimiter = ',')]
        relays: Vec<String>,

        /// Web URLs (can be specified multiple times)
        #[arg(long = "web", value_delimiter = ',')]
        web: Vec<String>,

        /// Private key in hex format or nsec bech32
        #[arg(long = "nsec", env = "NOSTR_PRIVATE_KEY")]
        private_key: String,

        /// Root commit (auto-detected if not provided)
        #[arg(long)]
        root_commit: Option<String>,

        /// Additional maintainer npubs
        #[arg(long = "maintainer")]
        maintainers: Vec<String>,

        /// Repository path (default: current directory)
        #[arg(long, default_value = ".")]
        repo_path: PathBuf,

        /// Timeout in seconds
        #[arg(long, default_value = "30")]
        timeout: u64,

        /// Output format
        #[arg(long, value_enum, default_value = "human")]
        output: OutputFormat,

        /// Update git config with nostr URL
        #[arg(long, default_value = "true")]
        update_git_config: bool,
    },

    /// Generate announcement JSON from existing repo
    Generate {
        /// Output file (stdout if not specified)
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,

        /// Repository path
        #[arg(long, default_value = ".")]
        repo_path: PathBuf,

        /// Include sample relays
        #[arg(long)]
        include_sample_relays: bool,
    },

    /// Get current git state
    State {
        /// Repository identifier
        #[arg(long)]
        identifier: String,

        /// Repository path
        #[arg(long, default_value = ".")]
        repo_path: PathBuf,

        /// Output format
        #[arg(long, value_enum, default_value = "json")]
        output: OutputFormat,
    },
}

#[derive(ValueEnum, Clone, Debug)]
enum OutputFormat {
    Human,
    Json,
    Minimal,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            identifier,
            name,
            description,
            clone_urls,
            relays,
            web,
            private_key,
            root_commit,
            maintainers,
            repo_path,
            timeout,
            output,
            update_git_config: update_config,
        } => {
            // Validate inputs
            if relays.is_empty() {
                bail!("At least one relay is required (--relay)");
            }

            if identifier.contains(' ') || identifier.contains('/') {
                bail!("Identifier must not contain spaces or slashes");
            }

            // Build announcement
            let mut announcement = if repo_path.exists() {
                detect_from_git(&repo_path).unwrap_or_else(|_| RepoAnnouncement {
                    identifier: identifier.clone(),
                    name: name.clone(),
                    description: description.clone(),
                    clone_urls: vec![],
                    relays: vec![],
                    web: vec![],
                    root_commit: String::new(),
                    maintainers: vec![],
                    grasp_servers: vec![],
                })
            } else {
                RepoAnnouncement {
                    identifier: identifier.clone(),
                    name: name.clone(),
                    description: description.clone(),
                    clone_urls: vec![],
                    relays: vec![],
                    web: vec![],
                    root_commit: String::new(),
                    maintainers: vec![],
                    grasp_servers: vec![],
                }
            };

            // Override with provided values
            announcement.identifier = identifier;
            announcement.name = name;
            announcement.description = description;
            if !clone_urls.is_empty() {
                announcement.clone_urls = clone_urls;
            }
            announcement.relays = relays;
            announcement.web = web;
            announcement.maintainers = maintainers;

            if let Some(commit) = root_commit {
                announcement.root_commit = commit;
            }

            // Ensure we have a root commit
            if announcement.root_commit.is_empty() {
                bail!("Root commit could not be detected. Please specify --root-commit");
            }

            // Clean up private key (remove nsec prefix if present)
            let clean_private_key = if private_key.starts_with("nsec") {
                // TODO: Properly decode nsec bech32
                bail!(
                    "nsec bech32 format not yet supported. Please provide private key in hex format"
                );
            } else {
                private_key
            };

            // Publish
            let config = PublishConfig {
                timeout_secs: timeout,
                wait_for_send: true,
            };

            let result = announce_repository(announcement.clone(), &clean_private_key, config)
                .await
                .context("Failed to announce repository")?;

            // Update git config if requested
            if update_config && repo_path.exists()
                && let Err(e) = update_git_config(&repo_path, &result.nostr_url) {
                    eprintln!("Warning: Failed to update git config: {}", e);
                }

            // Output result
            match output {
                OutputFormat::Human => {
                    println!("✅ Repository announced successfully!");
                    println!();
                    println!("Event ID: {}", result.event_id);
                    println!("Nostr URL: {}", result.nostr_url);
                    println!();
                    println!("Published to {} relays:", result.successes.len());
                    for relay in &result.successes {
                        println!("  ✓ {}", relay);
                    }
                    if !result.failures.is_empty() {
                        println!();
                        println!("⚠️  Failed relays:");
                        for (relay, error) in &result.failures {
                            println!("  ✗ {}: {}", relay, error);
                        }
                    }
                    println!();
                    println!("To clone this repository:");
                    println!("  git clone {}", result.nostr_url);
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                OutputFormat::Minimal => {
                    println!("{}", result.nostr_url);
                }
            }
        }

        Commands::Generate {
            output,
            repo_path,
            include_sample_relays,
        } => {
            let mut announcement =
                detect_from_git(&repo_path).context("Failed to detect repository information")?;

            if include_sample_relays {
                announcement.relays = vec![
                    "wss://relay.damus.io".to_string(),
                    "wss://nos.lol".to_string(),
                    "wss://relay.nostr.band".to_string(),
                ];
            }

            let json = serde_json::to_string_pretty(&announcement)?;

            if let Some(path) = output {
                std::fs::write(path, json)?;
                println!("Repository configuration written to file");
            } else {
                println!("{}", json);
            }
        }

        Commands::State {
            identifier,
            repo_path,
            output,
        } => {
            let state =
                get_git_state(&repo_path, &identifier).context("Failed to get git state")?;

            match output {
                OutputFormat::Json => {
                    let json = serde_json::json!({
                        "identifier": state.identifier,
                        "refs": state.refs
                    });
                    println!("{}", serde_json::to_string_pretty(&json)?);
                }
                OutputFormat::Human => {
                    println!("Git State for '{}':", state.identifier);
                    println!();
                    for (ref_name, commit) in &state.refs {
                        println!("  {} -> {}", ref_name, &commit[..8]);
                    }
                }
                OutputFormat::Minimal => {
                    for (ref_name, commit) in &state.refs {
                        println!("{}:{}", ref_name, commit);
                    }
                }
            }
        }
    }

    Ok(())
}
