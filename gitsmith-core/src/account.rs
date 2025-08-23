use anyhow::{Context, Result, bail};
use chacha20poly1305::{
    ChaCha20Poly1305, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use nostr::{Keys, ToBech32};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct StoredAccount {
    pub npub: String,
    pub encrypted_nsec: Vec<u8>,
    pub nonce: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountStorage {
    pub accounts: Vec<StoredAccount>,
    pub active_npub: Option<String>,
}

impl Default for AccountStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountStorage {
    pub fn new() -> Self {
        Self {
            accounts: Vec::new(),
            active_npub: None,
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let data = fs::read_to_string(path)
            .with_context(|| format!("Failed to read account storage from {:?}", path))?;

        serde_json::from_str(&data).context("Failed to parse account storage")
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {:?}", parent))?;
        }

        let data = serde_json::to_string_pretty(self)?;
        fs::write(path, data)
            .with_context(|| format!("Failed to write account storage to {:?}", path))
    }
}

/// Get the default account storage path
pub fn get_account_storage_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Failed to get home directory")?;
    Ok(home.join(".config").join("gitsmith").join("accounts.json"))
}

/// Derive encryption key from password
fn derive_key(password: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(b"gitsmith-account-encryption");
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Login with a private key and password
pub fn login(nsec_or_hex: &str, password: &str) -> Result<()> {
    // Parse the key (works with both nsec bech32 and hex format)
    let keys = Keys::parse(nsec_or_hex)?;

    let npub = keys.public_key().to_bech32()?;

    // Encrypt the private key
    let key = derive_key(password);
    let cipher = ChaCha20Poly1305::new_from_slice(&key)?;
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    let secret_key_bytes = keys.secret_key().to_secret_bytes();
    let encrypted = cipher
        .encrypt(&nonce, secret_key_bytes.as_ref())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {e}"))?;

    // Load existing storage
    let storage_path = get_account_storage_path()?;
    let mut storage = AccountStorage::load(&storage_path)?;

    // Check if account already exists
    if let Some(existing) = storage.accounts.iter_mut().find(|a| a.npub == npub) {
        // Update existing account
        existing.encrypted_nsec = encrypted;
        existing.nonce = nonce.to_vec();
    } else {
        // Add new account
        storage.accounts.push(StoredAccount {
            npub: npub.clone(),
            encrypted_nsec: encrypted,
            nonce: nonce.to_vec(),
        });
    }

    // Set as active account
    storage.active_npub = Some(npub.clone());

    // Save storage
    storage.save(&storage_path)?;

    println!("Logged in as {npub}");
    Ok(())
}

/// Logout (remove active account)
pub fn logout() -> Result<()> {
    let storage_path = get_account_storage_path()?;
    let mut storage = AccountStorage::load(&storage_path)?;

    if storage.active_npub.is_none() {
        bail!("No active account to logout");
    }

    let npub = storage.active_npub.take().unwrap();
    storage.save(&storage_path)?;

    println!("Logged out from {npub}");
    Ok(())
}

/// Get the active account keys
pub fn get_active_keys(password: &str) -> Result<Keys> {
    let storage_path = get_account_storage_path()?;
    let storage = AccountStorage::load(&storage_path)?;

    let active_npub = storage
        .active_npub
        .context("No active account. Please login first")?;

    let account = storage
        .accounts
        .iter()
        .find(|a| a.npub == active_npub)
        .context("Active account not found in storage")?;

    // Decrypt the private key
    let key = derive_key(password);
    let cipher = ChaCha20Poly1305::new_from_slice(&key)?;
    let nonce = Nonce::from_slice(&account.nonce);

    let decrypted = cipher
        .decrypt(nonce, account.encrypted_nsec.as_ref())
        .map_err(|_| anyhow::anyhow!("Failed to decrypt key. Wrong password?"))?;

    // Parse the decrypted key
    let hex_key = hex::encode(decrypted);
    Keys::parse(&hex_key).map_err(|e| anyhow::anyhow!("Failed to parse key: {e}"))
}

/// Export the active account
pub fn export_keys(password: &str) -> Result<String> {
    let keys = get_active_keys(password)?;
    Ok(keys.secret_key().to_bech32()?)
}

/// List all accounts
pub fn list_accounts() -> Result<Vec<String>> {
    let storage_path = get_account_storage_path()?;
    let storage = AccountStorage::load(&storage_path)?;

    Ok(storage
        .accounts
        .iter()
        .map(|a| {
            let active = storage.active_npub.as_ref() == Some(&a.npub);
            if active {
                format!("{npub} (active)", npub = a.npub)
            } else {
                a.npub.clone()
            }
        })
        .collect())
}
