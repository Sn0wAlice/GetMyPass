use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

fn default_auto_lock() -> u64 {
    300
}
fn default_clipboard_clear() -> u64 {
    30
}
fn default_true() -> bool {
    true
}
fn default_theme() -> String {
    "dark".to_string()
}
fn default_gen_length() -> usize {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_auto_lock")]
    pub auto_lock_seconds: u64,
    #[serde(default = "default_clipboard_clear")]
    pub clipboard_clear_seconds: u64,
    #[serde(default = "default_true")]
    pub backup_enabled: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_gen_length")]
    pub default_gen_length: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_lock_seconds: default_auto_lock(),
            clipboard_clear_seconds: default_clipboard_clear(),
            backup_enabled: default_true(),
            theme: default_theme(),
            default_gen_length: default_gen_length(),
        }
    }
}

/// Predefined auto-lock timeout options (in seconds)
pub const AUTO_LOCK_OPTIONS: &[(u64, &str)] = &[
    (0, "Off"),
    (60, "1 min"),
    (120, "2 min"),
    (300, "5 min"),
    (600, "10 min"),
    (900, "15 min"),
    (1800, "30 min"),
];

/// Predefined clipboard clear options (in seconds)
pub const CLIPBOARD_CLEAR_OPTIONS: &[(u64, &str)] = &[
    (0, "Off"),
    (10, "10s"),
    (15, "15s"),
    (30, "30s"),
    (60, "60s"),
];

impl Config {
    pub fn auto_lock_label(&self) -> &str {
        AUTO_LOCK_OPTIONS
            .iter()
            .find(|(v, _)| *v == self.auto_lock_seconds)
            .map(|(_, l)| *l)
            .unwrap_or("Custom")
    }

    pub fn clipboard_clear_label(&self) -> &str {
        CLIPBOARD_CLEAR_OPTIONS
            .iter()
            .find(|(v, _)| *v == self.clipboard_clear_seconds)
            .map(|(_, l)| *l)
            .unwrap_or("Custom")
    }

    pub fn cycle_auto_lock(&mut self, forward: bool) {
        let idx = AUTO_LOCK_OPTIONS
            .iter()
            .position(|(v, _)| *v == self.auto_lock_seconds)
            .unwrap_or(0);
        let new_idx = if forward {
            (idx + 1) % AUTO_LOCK_OPTIONS.len()
        } else {
            (idx + AUTO_LOCK_OPTIONS.len() - 1) % AUTO_LOCK_OPTIONS.len()
        };
        self.auto_lock_seconds = AUTO_LOCK_OPTIONS[new_idx].0;
    }

    pub fn cycle_clipboard_clear(&mut self, forward: bool) {
        let idx = CLIPBOARD_CLEAR_OPTIONS
            .iter()
            .position(|(v, _)| *v == self.clipboard_clear_seconds)
            .unwrap_or(0);
        let new_idx = if forward {
            (idx + 1) % CLIPBOARD_CLEAR_OPTIONS.len()
        } else {
            (idx + CLIPBOARD_CLEAR_OPTIONS.len() - 1) % CLIPBOARD_CLEAR_OPTIONS.len()
        };
        self.clipboard_clear_seconds = CLIPBOARD_CLEAR_OPTIONS[new_idx].0;
    }

    pub fn cycle_theme(&mut self) {
        let opts = crate::theme::THEME_OPTIONS;
        let idx = opts.iter().position(|&t| t == self.theme).unwrap_or(0);
        let new_idx = (idx + 1) % opts.len();
        self.theme = opts[new_idx].to_string();
    }

    pub fn adjust_gen_length(&mut self, forward: bool) {
        if forward {
            self.default_gen_length = (self.default_gen_length + 1).min(128);
        } else {
            self.default_gen_length = self.default_gen_length.saturating_sub(1).max(4);
        }
    }
}

pub fn config_path() -> PathBuf {
    crate::vault::vault_dir().join("config.toml")
}

pub fn load_config() -> Config {
    let path = config_path();
    if !path.exists() {
        return Config::default();
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
    toml::from_str(&content).unwrap_or_default()
}

pub fn save_config(config: &Config) -> Result<(), String> {
    crate::vault::ensure_vault_dir();
    let path = config_path();
    let content =
        toml::to_string_pretty(config).map_err(|e| format!("Config serialize error: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Config write error: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.auto_lock_seconds, 300);
        assert_eq!(config.clipboard_clear_seconds, 30);
        assert!(config.backup_enabled);
        assert_eq!(config.theme, "dark");
        assert_eq!(config.default_gen_length, 20);
    }

    #[test]
    fn test_cycle_auto_lock() {
        let mut config = Config::default();
        config.cycle_auto_lock(true);
        assert_eq!(config.auto_lock_seconds, 600);
        config.cycle_auto_lock(false);
        assert_eq!(config.auto_lock_seconds, 300);
    }

    #[test]
    fn test_cycle_clipboard_clear() {
        let mut config = Config::default();
        config.cycle_clipboard_clear(true);
        assert_eq!(config.clipboard_clear_seconds, 60);
    }

    #[test]
    fn test_cycle_theme() {
        let mut config = Config::default();
        assert_eq!(config.theme, "dark");
        config.cycle_theme();
        assert_eq!(config.theme, "light");
        config.cycle_theme();
        assert_eq!(config.theme, "dark");
    }

    #[test]
    fn test_gen_length_bounds() {
        let mut config = Config::default();
        config.default_gen_length = 4;
        config.adjust_gen_length(false);
        assert_eq!(config.default_gen_length, 4);
        config.default_gen_length = 128;
        config.adjust_gen_length(true);
        assert_eq!(config.default_gen_length, 128);
    }

    #[test]
    fn test_serde_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.auto_lock_seconds, config.auto_lock_seconds);
        assert_eq!(parsed.theme, config.theme);
    }

    #[test]
    fn test_partial_config_compat() {
        // Old config without new fields should deserialize with defaults
        let toml_str = "auto_lock_seconds = 600\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.auto_lock_seconds, 600);
        assert_eq!(config.theme, "dark"); // default
        assert_eq!(config.default_gen_length, 20); // default
    }
}
