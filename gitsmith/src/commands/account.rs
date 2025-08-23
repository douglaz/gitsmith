use anyhow::Result;
use clap::Subcommand;
use gitsmith_core::account;
use rpassword::read_password;
use std::io::{self, Write};

#[derive(Subcommand)]
pub enum AccountCommands {
    /// Login with a private key
    Login {
        /// nsec or hex private key
        #[arg(long)]
        nsec: String,
    },
    /// Logout from active account
    Logout,
    /// Export active account private key
    Export,
    /// List all accounts
    List,
}

pub async fn handle_account_command(command: AccountCommands) -> Result<()> {
    match command {
        AccountCommands::Login { nsec } => {
            print!("Enter password to encrypt key: ");
            io::stdout().flush()?;
            let password = read_password()?;

            account::login(&nsec, &password)?;
            Ok(())
        }
        AccountCommands::Logout => {
            account::logout()?;
            Ok(())
        }
        AccountCommands::Export => {
            print!("Enter password to decrypt key: ");
            io::stdout().flush()?;
            let password = read_password()?;

            let nsec = account::export_keys(&password)?;
            println!("Private key: {}", nsec);
            Ok(())
        }
        AccountCommands::List => {
            let accounts = account::list_accounts()?;
            if accounts.is_empty() {
                println!("No accounts found");
            } else {
                println!("Accounts:");
                for account in accounts {
                    println!("  {}", account);
                }
            }
            Ok(())
        }
    }
}
