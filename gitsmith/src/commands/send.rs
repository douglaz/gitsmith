use anyhow::Result;
use clap::Args;
use gitsmith_core::{account, patches};
use nostr_sdk::Client;
use rpassword::read_password;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Args)]
pub struct SendArgs {
    /// Commits to send (e.g., HEAD~2)
    #[arg(default_value = "HEAD~1")]
    pub since: String,
    
    /// Title for the pull request
    #[arg(long, short = 't')]
    pub title: Option<String>,
    
    /// Description for the pull request
    #[arg(long, short = 'd')]
    pub description: Option<String>,
    
    /// Reply to an existing PR (event ID)
    #[arg(long)]
    pub in_reply_to: Option<String>,
    
    /// Repository path
    #[arg(long, default_value = ".")]
    pub repo_path: PathBuf,
}

pub async fn handle_send_command(args: SendArgs) -> Result<()> {
    // Get account keys
    print!("Enter password: ");
    io::stdout().flush()?;
    let password = read_password()?;
    let keys = account::get_active_keys(&password)?;
    
    // Get repository info
    let repo_announcement = gitsmith_core::detect_from_git(&args.repo_path)?;
    
    // Generate patches
    println!("Generating patches from {}...", args.since);
    let patches = patches::generate_patches(&args.repo_path, Some(&args.since), None)?;
    
    if patches.is_empty() {
        println!("No patches to send");
        return Ok(());
    }
    
    println!("Generated {} patch(es)", patches.len());
    
    // Get title and description
    let title = if let Some(t) = args.title {
        t
    } else {
        print!("Enter PR title: ");
        io::stdout().flush()?;
        let mut title = String::new();
        io::stdin().read_line(&mut title)?;
        title.trim().to_string()
    };
    
    let description = if let Some(d) = args.description {
        d
    } else {
        print!("Enter PR description (optional): ");
        io::stdout().flush()?;
        let mut desc = String::new();
        io::stdin().read_line(&mut desc)?;
        desc.trim().to_string()
    };
    
    // Create repository coordinate
    let repo_coordinate = format!(
        "30617:{}:{}",
        keys.public_key(),
        repo_announcement.identifier
    );
    
    // Create PR events
    let events = patches::create_pull_request_event(
        &keys,
        &repo_coordinate,
        &title,
        &description,
        patches,
        &repo_announcement.root_commit,
        args.in_reply_to,
    )?;
    
    println!("Created {} events", events.len());
    
    // Send to relays
    if repo_announcement.relays.is_empty() {
        println!("Warning: No relays configured for repository");
        println!("Please run 'gitsmith init' first to configure relays");
        return Ok(());
    }
    
    let client = Client::new(keys.clone());
    
    for relay_url in &repo_announcement.relays {
        client.add_relay(relay_url).await?;
    }
    
    client.connect().await;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    
    println!("Sending PR to {} relay(s)...", repo_announcement.relays.len());
    
    for event in events {
        client.send_event(&event).await?;
    }
    
    println!("✅ Pull request sent successfully!");
    
    Ok(())
}