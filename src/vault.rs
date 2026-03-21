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
pub const MAX_PASSWORD_HISTORY: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntryKind {
    Password,
    Note,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordHistoryItem {
    pub password: String,
    pub changed_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: Uuid,
    pub kind: EntryKind,
    #[serde(default)]
    pub folder: String,
    pub name: String,
    pub username: String,
    pub password: String,
    pub url: String,
    pub notes: String,
    pub created_at: i64,
    pub modified_at: i64,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub totp_secret: String,
    #[serde(default)]
    pub password_history: Vec<PasswordHistoryItem>,
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
            favorite: false,
            tags: Vec::new(),
            totp_secret: String::new(),
            password_history: Vec::new(),
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
            favorite: false,
            tags: Vec::new(),
            totp_secret: String::new(),
            password_history: Vec::new(),
        }
    }

    pub fn matches(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        self.name.to_lowercase().contains(&q)
            || self.username.to_lowercase().contains(&q)
            || self.url.to_lowercase().contains(&q)
            || self.notes.to_lowercase().contains(&q)
            || self.folder.to_lowercase().contains(&q)
            || self.tags.iter().any(|t| t.to_lowercase().contains(&q))
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
}

/// Password strength score (0-4)
pub fn password_strength_score(password: &str) -> (u8, &'static str, &'static str) {
    let len = password.len();
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_symbol = password.chars().any(|c| !c.is_alphanumeric());
    let variety = [has_upper, has_lower, has_digit, has_symbol]
        .iter()
        .filter(|&&x| x)
        .count();

    if len >= 16 && variety >= 3 {
        (4, "Strong", "||||")
    } else if len >= 12 && variety >= 2 {
        (3, "Good", "|||")
    } else if len >= 8 && variety >= 2 {
        (2, "Fair", "||")
    } else if len >= 8 {
        (1, "Weak", "|")
    } else {
        (0, "Too short", "")
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
    save_vault_with_backup(vault, password, true)
}

pub fn save_vault_with_backup(
    vault: &Vault,
    password: &str,
    backup: bool,
) -> Result<(), String> {
    ensure_vault_dir();
    let path = vault_path();

    if backup && path.exists() {
        let backup_path = path.with_extension("enc.bak");
        let _ = fs::copy(&path, &backup_path);
    }

    let data = encrypt_vault(vault, password);
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, &data).map_err(|e| format!("Failed to write vault: {}", e))?;
    fs::rename(&tmp_path, &path).map_err(|e| format!("Failed to save vault: {}", e))?;
    Ok(())
}

pub fn export_vault_json(vault: &Vault) -> Result<String, String> {
    let export_path = vault_dir().join("export.json");
    let json = serde_json::to_string_pretty(&vault.entries)
        .map_err(|e| format!("Serialize error: {}", e))?;
    fs::write(&export_path, &json).map_err(|e| format!("Write error: {}", e))?;
    Ok(export_path.to_string_lossy().to_string())
}

pub fn import_vault_json(vault: &mut Vault, file_path: &str) -> Result<usize, String> {
    let content =
        fs::read_to_string(file_path).map_err(|e| format!("Failed to read file: {}", e))?;
    let entries: Vec<Entry> =
        serde_json::from_str(&content).map_err(|e| format!("Invalid JSON format: {}", e))?;
    let count = entries.len();
    for mut entry in entries {
        entry.id = Uuid::new_v4();
        vault.entries.push(entry);
    }
    Ok(count)
}

pub fn change_master_password(vault: &Vault, new_password: &str) -> Result<(), String> {
    save_vault(vault, new_password)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let mut vault = Vault::new();
        let mut entry = Entry::new_password();
        entry.name = "Test".to_string();
        entry.username = "user@example.com".to_string();
        entry.password = "supersecret123!".to_string();
        entry.url = "https://example.com".to_string();
        vault.entries.push(entry);

        let password = "testmaster12345";
        let encrypted = encrypt_vault(&vault, password);
        let decrypted = decrypt_vault(&encrypted, password).unwrap();

        assert_eq!(decrypted.entries.len(), 1);
        assert_eq!(decrypted.entries[0].name, "Test");
        assert_eq!(decrypted.entries[0].username, "user@example.com");
        assert_eq!(decrypted.entries[0].password, "supersecret123!");
    }

    #[test]
    fn test_decrypt_wrong_password() {
        let vault = Vault::new();
        let encrypted = encrypt_vault(&vault, "correctpassword");
        let result = decrypt_vault(&encrypted, "wrongpassword");
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_corrupted_data() {
        let result = decrypt_vault(&[0u8; 10], "password");
        assert!(result.is_err());
    }

    #[test]
    fn test_password_strength_too_short() {
        let (score, label, _) = password_strength_score("abc");
        assert_eq!(score, 0);
        assert_eq!(label, "Too short");
    }

    #[test]
    fn test_password_strength_weak() {
        let (score, label, _) = password_strength_score("abcdefgh");
        assert_eq!(score, 1);
        assert_eq!(label, "Weak");
    }

    #[test]
    fn test_password_strength_fair() {
        let (score, label, _) = password_strength_score("abcdefG1");
        assert_eq!(score, 2);
        assert_eq!(label, "Fair");
    }

    #[test]
    fn test_password_strength_good() {
        let (score, label, _) = password_strength_score("abcdefghijG1");
        assert_eq!(score, 3);
        assert_eq!(label, "Good");
    }

    #[test]
    fn test_password_strength_strong() {
        let (score, label, _) = password_strength_score("Abcdefghijklmno1!");
        assert_eq!(score, 4);
        assert_eq!(label, "Strong");
    }

    #[test]
    fn test_entry_matches_name() {
        let mut entry = Entry::new_password();
        entry.name = "GitHub".to_string();
        assert!(entry.matches("git"));
        assert!(entry.matches("GITHUB"));
        assert!(!entry.matches("gitlab"));
    }

    #[test]
    fn test_entry_matches_username() {
        let mut entry = Entry::new_password();
        entry.username = "alice@example.com".to_string();
        assert!(entry.matches("alice"));
        assert!(entry.matches("example"));
    }

    #[test]
    fn test_entry_matches_tags() {
        let mut entry = Entry::new_password();
        entry.tags = vec!["work".to_string(), "dev".to_string()];
        assert!(entry.matches("work"));
        assert!(entry.matches("dev"));
        assert!(!entry.matches("personal"));
    }

    #[test]
    fn test_entry_matches_folder() {
        let mut entry = Entry::new_password();
        entry.folder = "Work/Email".to_string();
        assert!(entry.matches("work"));
        assert!(entry.matches("email"));
    }

    #[test]
    fn test_new_vault_empty() {
        let vault = Vault::new();
        assert!(vault.entries.is_empty());
    }

    #[test]
    fn test_entry_serde_backward_compat() {
        // Simulate old vault entry JSON without newer fields
        let json = r#"{
            "id": "00000000-0000-0000-0000-000000000001",
            "kind": "Password",
            "name": "Old Entry",
            "username": "user",
            "password": "pass",
            "url": "",
            "notes": "",
            "created_at": 1700000000,
            "modified_at": 1700000000
        }"#;
        let entry: Entry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.name, "Old Entry");
        assert!(entry.folder.is_empty());
        assert!(!entry.favorite);
        assert!(entry.tags.is_empty());
        assert!(entry.totp_secret.is_empty());
        assert!(entry.password_history.is_empty());
    }

    #[test]
    fn test_encrypt_different_each_time() {
        let vault = Vault::new();
        let password = "testpassword";
        let enc1 = encrypt_vault(&vault, password);
        let enc2 = encrypt_vault(&vault, password);
        // Different salt/nonce means different ciphertext
        assert_ne!(enc1, enc2);
    }
}
