use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;
use zeroize::Zeroize;

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntryKind {
    Password,
    Note,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: Uuid,
    pub kind: EntryKind,
    pub folder: String, // e.g. "Work/Email" or "" for root
    pub name: String,
    pub username: String,
    pub password: String,
    pub url: String,
    pub notes: String,
    pub created_at: i64,
    pub modified_at: i64,
}

impl Entry {
    pub fn new_password() -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: Uuid::new_v4(),
            kind: EntryKind::Password,
            folder: String::new(),
            name: String::new(),
            username: String::new(),
            password: String::new(),
            url: String::new(),
            notes: String::new(),
            created_at: now,
            modified_at: now,
        }
    }

    pub fn new_note() -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: Uuid::new_v4(),
            kind: EntryKind::Note,
            folder: String::new(),
            name: String::new(),
            username: String::new(),
            password: String::new(),
            url: String::new(),
            notes: String::new(),
            created_at: now,
            modified_at: now,
        }
    }

    pub fn matches(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        self.name.to_lowercase().contains(&q)
            || self.username.to_lowercase().contains(&q)
            || self.url.to_lowercase().contains(&q)
            || self.notes.to_lowercase().contains(&q)
            || self.folder.to_lowercase().contains(&q)
    }

    /// Returns folder depth (0 = root, max 3)
    pub fn folder_depth(&self) -> usize {
        if self.folder.is_empty() {
            0
        } else {
            self.folder.matches('/').count() + 1
        }
    }

    /// Returns the display path: "folder/name" or just "name"
    pub fn display_path(&self) -> String {
        if self.folder.is_empty() {
            self.name.clone()
        } else {
            format!("{}/{}", self.folder, self.name)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Vault {
    pub entries: Vec<Entry>,
}

impl Vault {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Collect all unique folders used in the vault
    pub fn folders(&self) -> Vec<String> {
        let mut folders: Vec<String> = self
            .entries
            .iter()
            .filter(|e| !e.folder.is_empty())
            .map(|e| e.folder.clone())
            .collect();
        folders.sort();
        folders.dedup();
        // Also add parent folders
        let mut all: Vec<String> = Vec::new();
        for f in &folders {
            let parts: Vec<&str> = f.split('/').collect();
            let mut path = String::new();
            for (i, part) in parts.iter().enumerate() {
                if i > 0 {
                    path.push('/');
                }
                path.push_str(part);
                if !all.contains(&path) {
                    all.push(path.clone());
                }
            }
        }
        all.sort();
        all
    }
}

fn derive_key(password: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    let argon2 = Argon2::default();
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .expect("Argon2 key derivation failed");
    key
}

pub fn encrypt_vault(vault: &Vault, password: &str) -> Vec<u8> {
    let json = serde_json::to_vec(vault).expect("Failed to serialize vault");

    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);

    let mut key = derive_key(password, &salt);
    let cipher = Aes256Gcm::new_from_slice(&key).expect("Invalid key length");
    key.zeroize();

    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, json.as_ref()).expect("Encryption failed");

    let mut output = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);
    output
}

pub fn decrypt_vault(data: &[u8], password: &str) -> Result<Vault, String> {
    if data.len() < SALT_LEN + NONCE_LEN + 16 {
        return Err("Vault file is too small or corrupted".into());
    }

    let salt = &data[..SALT_LEN];
    let nonce_bytes = &data[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &data[SALT_LEN + NONCE_LEN..];

    let mut key = derive_key(password, salt);
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| format!("Invalid key: {}", e))?;
    key.zeroize();

    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Wrong master password or corrupted vault".to_string())?;

    serde_json::from_slice(&plaintext).map_err(|e| format!("Failed to parse vault: {}", e))
}

pub fn vault_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot determine home directory");
    home.join(".gmp")
}

pub fn vault_path() -> PathBuf {
    vault_dir().join("vault.enc")
}

pub fn ensure_vault_dir() {
    let dir = vault_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).expect("Failed to create ~/.gmp directory");
    }
}

pub fn load_vault(password: &str) -> Result<Vault, String> {
    let path = vault_path();
    if !path.exists() {
        return Ok(Vault::new());
    }
    let data = fs::read(&path).map_err(|e| format!("Failed to read vault file: {}", e))?;
    decrypt_vault(&data, password)
}

pub fn save_vault(vault: &Vault, password: &str) -> Result<(), String> {
    ensure_vault_dir();
    let data = encrypt_vault(vault, password);
    let path = vault_path();
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, &data).map_err(|e| format!("Failed to write vault: {}", e))?;
    fs::rename(&tmp_path, &path).map_err(|e| format!("Failed to save vault: {}", e))?;
    Ok(())
}
