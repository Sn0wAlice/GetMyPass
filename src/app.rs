use crate::config::Config;
use crate::theme::Theme;
use crate::vault::{Entry, EntryKind, PasswordHistoryItem, Vault, MAX_PASSWORD_HISTORY};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Vault,
    Settings,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    List,
    ViewEntry,
    EditEntry,
    ConfirmDelete,
    GeneratePassword,
    Settings,
    ChangePassword,
    Locked,
    ImportPath,
    Stats,
    InitialUnlock,
    InitialSetup,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
    Editing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortMode {
    Name,
    DateCreated,
    DateModified,
}

impl SortMode {
    pub fn label(&self) -> &str {
        match self {
            SortMode::Name => "A-Z",
            SortMode::DateCreated => "Created",
            SortMode::DateModified => "Modified",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            SortMode::Name => SortMode::DateCreated,
            SortMode::DateCreated => SortMode::DateModified,
            SortMode::DateModified => SortMode::Name,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntryField {
    Folder,
    Name,
    Username,
    Password,
    Url,
    TotpSecret,
    Tags,
    Notes,
}

impl EntryField {
    pub fn all_password() -> &'static [EntryField] {
        &[
            EntryField::Folder,
            EntryField::Name,
            EntryField::Username,
            EntryField::Password,
            EntryField::Url,
            EntryField::TotpSecret,
            EntryField::Tags,
            EntryField::Notes,
        ]
    }

    pub fn all_note() -> &'static [EntryField] {
        &[
            EntryField::Folder,
            EntryField::Name,
            EntryField::Tags,
            EntryField::Notes,
        ]
    }

    pub fn label(&self) -> &str {
        match self {
            EntryField::Folder => "Folder",
            EntryField::Name => "Name",
            EntryField::Username => "Username",
            EntryField::Password => "Password",
            EntryField::Url => "URL",
            EntryField::TotpSecret => "TOTP Secret",
            EntryField::Tags => "Tags",
            EntryField::Notes => "Notes",
        }
    }
}

/// Represents a row in the list view
#[derive(Debug, Clone)]
pub enum ListRow {
    Folder(String),
    Entry(usize),
}

/// Settings menu items
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsItem {
    AutoLock,
    ClipboardClear,
    AutoBackup,
    ThemeSetting,
    DefaultGenLength,
    ChangePassword,
    ExportJson,
    ImportJson,
}

pub const SETTINGS_ITEMS: &[SettingsItem] = &[
    SettingsItem::AutoLock,
    SettingsItem::ClipboardClear,
    SettingsItem::AutoBackup,
    SettingsItem::ThemeSetting,
    SettingsItem::DefaultGenLength,
    SettingsItem::ChangePassword,
    SettingsItem::ExportJson,
    SettingsItem::ImportJson,
];

impl SettingsItem {
    pub fn label(&self) -> &str {
        match self {
            SettingsItem::AutoLock => "Auto-lock timeout",
            SettingsItem::ClipboardClear => "Clipboard auto-clear",
            SettingsItem::AutoBackup => "Auto-backup on save",
            SettingsItem::ThemeSetting => "Theme",
            SettingsItem::DefaultGenLength => "Default password length",
            SettingsItem::ChangePassword => "Change master password",
            SettingsItem::ExportJson => "Export vault (JSON)",
            SettingsItem::ImportJson => "Import entries (JSON)",
        }
    }

    pub fn section(&self) -> &str {
        match self {
            SettingsItem::AutoLock
            | SettingsItem::ClipboardClear
            | SettingsItem::AutoBackup => "Security",
            SettingsItem::ThemeSetting | SettingsItem::DefaultGenLength => "Display",
            SettingsItem::ChangePassword => "Password",
            SettingsItem::ExportJson | SettingsItem::ImportJson => "Data",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SetupStep {
    NewPassword,
    ConfirmPassword,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PasswordChangeStep {
    CurrentPassword,
    NewPassword,
    ConfirmPassword,
}

/// Vault statistics
pub struct VaultStats {
    pub total: usize,
    pub passwords: usize,
    pub notes: usize,
    pub favorites: usize,
    pub folders: usize,
    pub weak_passwords: usize,
    pub duplicate_passwords: usize,
    pub avg_age_days: u64,
    pub oldest_days: u64,
    pub tags_count: usize,
    pub totp_count: usize,
}

pub struct App {
    // Core
    pub vault: Vault,
    pub master_password: String,
    pub config: Config,
    pub theme: Theme,

    // Tab system
    pub active_tab: Tab,

    // Screen
    pub screen: Screen,
    pub input_mode: InputMode,
    pub should_quit: bool,
    pub dirty: bool,

    // Vault list
    pub search_query: String,
    pub filtered_indices: Vec<usize>,
    pub list_rows: Vec<ListRow>,
    pub selected: usize,
    pub sort_mode: SortMode,

    // Entry editing
    pub edit_buffer: Entry,
    pub edit_is_new: bool,
    pub active_field: usize,
    pub show_password: bool,
    pub edit_tags_buffer: String,

    // Status
    pub status_message: Option<(String, Instant)>,

    // Folder navigation
    pub current_folder: String,
    pub collapsed_folders: Vec<String>,

    // Password generator
    pub gen_length: usize,
    pub gen_uppercase: bool,
    pub gen_lowercase: bool,
    pub gen_digits: bool,
    pub gen_symbols: bool,
    pub gen_preview: String,

    // Settings
    pub settings_selected: usize,

    // Password change
    pub pw_change_step: PasswordChangeStep,
    pub pw_change_current: String,
    pub pw_change_new: String,
    pub pw_change_confirm: String,
    pub pw_change_error: Option<String>,

    // Auto-lock
    pub last_activity: Instant,
    pub locked: bool,
    pub lock_password_input: String,
    pub lock_attempts: u8,
    pub lock_error: Option<String>,

    // Clipboard auto-clear
    pub clipboard_clear_at: Option<Instant>,

    // Import
    pub import_path_input: String,

    // View entry
    pub show_history: bool,

    // Initial unlock / setup
    pub initial_password_input: String,
    pub initial_password_confirm: String,
    pub initial_setup_step: SetupStep,
    pub initial_error: Option<String>,
    pub initial_attempts: u8,
}

impl App {
    pub fn new_locked(vault_exists: bool, config: Config) -> Self {
        let now = Instant::now();
        let theme = Theme::from_name(&config.theme);
        let gen_length = config.default_gen_length;
        let screen = if vault_exists {
            Screen::InitialUnlock
        } else {
            Screen::InitialSetup
        };
        Self {
            vault: Vault::new(),
            master_password: String::new(),
            config,
            theme,
            active_tab: Tab::Vault,
            screen,
            input_mode: InputMode::Normal,
            should_quit: false,
            dirty: false,
            search_query: String::new(),
            filtered_indices: Vec::new(),
            list_rows: Vec::new(),
            selected: 0,
            sort_mode: SortMode::Name,
            edit_buffer: Entry::new_password(),
            edit_is_new: true,
            active_field: 0,
            show_password: false,
            edit_tags_buffer: String::new(),
            status_message: None,
            current_folder: String::new(),
            collapsed_folders: Vec::new(),
            gen_length,
            gen_uppercase: true,
            gen_lowercase: true,
            gen_digits: true,
            gen_symbols: true,
            gen_preview: String::new(),
            settings_selected: 0,
            pw_change_step: PasswordChangeStep::CurrentPassword,
            pw_change_current: String::new(),
            pw_change_new: String::new(),
            pw_change_confirm: String::new(),
            pw_change_error: None,
            last_activity: now,
            locked: false,
            lock_password_input: String::new(),
            lock_attempts: 0,
            lock_error: None,
            clipboard_clear_at: None,
            import_path_input: String::new(),
            show_history: false,
            initial_password_input: String::new(),
            initial_password_confirm: String::new(),
            initial_setup_step: SetupStep::NewPassword,
            initial_error: None,
            initial_attempts: 0,
        }
    }

    pub fn finalize_unlock(&mut self, vault: Vault, master_password: String) {
        self.vault = vault;
        self.master_password = master_password;
        self.initial_password_input.clear();
        self.initial_password_confirm.clear();
        self.initial_error = None;
        self.screen = Screen::List;
        self.input_mode = InputMode::Normal;
        self.active_tab = Tab::Vault;
        self.last_activity = Instant::now();
        self.update_filter();
    }

    pub fn touch_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn check_auto_lock(&mut self) {
        if self.config.auto_lock_seconds == 0
            || self.locked
            || matches!(self.screen, Screen::InitialUnlock | Screen::InitialSetup)
        {
            return;
        }
        if self.last_activity.elapsed().as_secs() >= self.config.auto_lock_seconds {
            self.lock();
        }
    }

    pub fn lock(&mut self) {
        self.locked = true;
        self.screen = Screen::Locked;
        self.lock_password_input.clear();
        self.lock_attempts = 0;
        self.lock_error = None;
        self.show_password = false;
    }

    pub fn try_unlock(&mut self) -> bool {
        if self.lock_password_input == self.master_password {
            self.locked = false;
            self.screen = Screen::List;
            self.input_mode = InputMode::Normal;
            self.active_tab = Tab::Vault;
            self.lock_password_input.clear();
            self.lock_attempts = 0;
            self.lock_error = None;
            self.last_activity = Instant::now();
            true
        } else {
            self.lock_attempts += 1;
            self.lock_password_input.clear();
            if self.lock_attempts >= 3 {
                self.should_quit = true;
                false
            } else {
                self.lock_error = Some(format!(
                    "Wrong password ({}/3 attempts)",
                    self.lock_attempts
                ));
                false
            }
        }
    }

    pub fn schedule_clipboard_clear(&mut self) {
        if self.config.clipboard_clear_seconds > 0 {
            self.clipboard_clear_at = Some(
                Instant::now()
                    + std::time::Duration::from_secs(self.config.clipboard_clear_seconds),
            );
        }
    }

    pub fn check_clipboard_clear(&mut self) {
        if let Some(clear_at) = self.clipboard_clear_at {
            if Instant::now() >= clear_at {
                let _ = crate::clipboard::clear_clipboard();
                self.clipboard_clear_at = None;
                self.set_status("Clipboard auto-cleared");
            }
        }
    }

    // ─── Sort ─────────────────────────────────────────────────────

    pub fn cycle_sort(&mut self) {
        self.sort_mode = self.sort_mode.next();
        self.update_filter();
        self.set_status(format!("Sort: {}", self.sort_mode.label()));
    }

    fn sort_entries_vec(&self, entries: &mut [usize]) {
        sort_indices(&self.vault.entries, self.sort_mode, entries);
    }

    // ─── Filter ───────────────────────────────────────────────────

    pub fn update_filter(&mut self) {
        let is_searching = !self.search_query.is_empty();

        if is_searching {
            self.filtered_indices = self
                .vault
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| e.matches(&self.search_query))
                .map(|(i, _)| i)
                .collect();
            sort_indices(&self.vault.entries, self.sort_mode, &mut self.filtered_indices);
            self.list_rows = self
                .filtered_indices
                .iter()
                .map(|&i| ListRow::Entry(i))
                .collect();
        } else {
            self.build_folder_rows();
        }

        if self.selected >= self.list_rows.len() {
            self.selected = self.list_rows.len().saturating_sub(1);
        }
    }

    fn build_folder_rows(&mut self) {
        let mut rows: Vec<ListRow> = Vec::new();

        let mut folder_entries: Vec<(String, usize)> = Vec::new();
        let mut direct_subfolders: Vec<String> = Vec::new();

        for (i, entry) in self.vault.entries.iter().enumerate() {
            if self.current_folder.is_empty() {
                if entry.folder.is_empty() {
                    folder_entries.push((String::new(), i));
                } else {
                    let top = entry.folder.split('/').next().unwrap_or("").to_string();
                    if !direct_subfolders.contains(&top) {
                        direct_subfolders.push(top);
                    }
                    if !self.is_folder_collapsed(&top_folder(&entry.folder)) {
                        folder_entries.push((entry.folder.clone(), i));
                    }
                }
            } else if entry.folder == self.current_folder {
                folder_entries.push((entry.folder.clone(), i));
            } else if entry.folder.starts_with(&format!("{}/", self.current_folder)) {
                let remainder = &entry.folder[self.current_folder.len() + 1..];
                let sub = remainder.split('/').next().unwrap_or("").to_string();
                let full_sub = format!("{}/{}", self.current_folder, sub);
                if !direct_subfolders.contains(&full_sub) {
                    direct_subfolders.push(full_sub);
                }
                if !self.is_folder_collapsed(&entry.folder) {
                    folder_entries.push((entry.folder.clone(), i));
                }
            }
        }

        direct_subfolders.sort();

        for subfolder in &direct_subfolders {
            rows.push(ListRow::Folder(subfolder.clone()));

            if !self.is_folder_collapsed(subfolder) {
                let mut sub_entries: Vec<usize> = folder_entries
                    .iter()
                    .filter(|(f, _)| f == subfolder || f.starts_with(&format!("{}/", subfolder)))
                    .map(|(_, i)| *i)
                    .collect();
                self.sort_entries_vec(&mut sub_entries);
                for idx in sub_entries {
                    rows.push(ListRow::Entry(idx));
                }
            }
        }

        let mut root_entries: Vec<usize> = folder_entries
            .iter()
            .filter(|(f, _)| {
                if self.current_folder.is_empty() {
                    f.is_empty()
                } else {
                    *f == self.current_folder
                }
            })
            .map(|(_, i)| *i)
            .collect();
        self.sort_entries_vec(&mut root_entries);
        for idx in root_entries {
            rows.push(ListRow::Entry(idx));
        }

        self.list_rows = rows;
        self.filtered_indices = self
            .list_rows
            .iter()
            .filter_map(|r| match r {
                ListRow::Entry(i) => Some(*i),
                _ => None,
            })
            .collect();
    }

    fn is_folder_collapsed(&self, folder: &str) -> bool {
        self.collapsed_folders.iter().any(|f| f == folder)
    }

    pub fn toggle_folder_collapse(&mut self) {
        if let Some(ListRow::Folder(folder)) = self.list_rows.get(self.selected) {
            let folder = folder.clone();
            if let Some(pos) = self.collapsed_folders.iter().position(|f| *f == folder) {
                self.collapsed_folders.remove(pos);
            } else {
                self.collapsed_folders.push(folder);
            }
            self.update_filter();
        }
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        match self.list_rows.get(self.selected) {
            Some(ListRow::Entry(i)) => self.vault.entries.get(*i),
            _ => None,
        }
    }

    pub fn selected_entry_index(&self) -> Option<usize> {
        match self.list_rows.get(self.selected) {
            Some(ListRow::Entry(i)) => Some(*i),
            _ => None,
        }
    }

    pub fn selected_is_folder(&self) -> bool {
        matches!(self.list_rows.get(self.selected), Some(ListRow::Folder(_)))
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    pub fn clear_expired_status(&mut self) {
        if let Some((_, when)) = &self.status_message {
            if when.elapsed().as_secs() >= 3 {
                self.status_message = None;
            }
        }
    }

    // ─── Entry operations ─────────────────────────────────────────

    pub fn start_new_entry(&mut self, kind: EntryKind) {
        self.edit_buffer = match kind {
            EntryKind::Password => Entry::new_password(),
            EntryKind::Note => Entry::new_note(),
        };
        self.edit_buffer.folder = self.current_folder.clone();
        self.edit_is_new = true;
        self.active_field = 0;
        self.edit_tags_buffer.clear();
        self.screen = Screen::EditEntry;
        self.input_mode = InputMode::Editing;
    }

    pub fn start_edit_entry(&mut self) {
        if let Some(entry) = self.selected_entry() {
            self.edit_buffer = entry.clone();
            self.edit_is_new = false;
            self.active_field = 0;
            self.edit_tags_buffer = self.edit_buffer.tags.join(", ");
            self.screen = Screen::EditEntry;
            self.input_mode = InputMode::Editing;
        }
    }

    pub fn duplicate_selected(&mut self) {
        if let Some(entry) = self.selected_entry() {
            let mut dup = entry.clone();
            dup.id = uuid::Uuid::new_v4();
            dup.name = format!("{} (copy)", dup.name);
            let now = chrono::Utc::now().timestamp();
            dup.created_at = now;
            dup.modified_at = now;
            dup.password_history.clear();
            self.vault.entries.push(dup);
            self.dirty = true;
            self.update_filter();
            self.set_status("Entry duplicated");
        }
    }

    pub fn toggle_favorite(&mut self) {
        if let Some(idx) = self.selected_entry_index() {
            self.vault.entries[idx].favorite = !self.vault.entries[idx].favorite;
            let is_fav = self.vault.entries[idx].favorite;
            self.dirty = true;
            self.update_filter();
            self.set_status(if is_fav { "Added to favorites" } else { "Removed from favorites" });
        }
    }

    pub fn save_edit(&mut self) {
        let depth = if self.edit_buffer.folder.is_empty() {
            0
        } else {
            self.edit_buffer.folder.matches('/').count() + 1
        };
        if depth > 3 {
            self.set_status("Folder depth limited to 3 levels");
            return;
        }

        self.edit_buffer.folder = self.edit_buffer.folder.trim_matches('/').to_string();

        // Parse tags from buffer
        self.edit_buffer.tags = self
            .edit_tags_buffer
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Clean TOTP secret
        self.edit_buffer.totp_secret = self
            .edit_buffer
            .totp_secret
            .trim()
            .replace(' ', "")
            .replace('-', "")
            .to_uppercase();

        // Password history: save old password before overwriting
        if !self.edit_is_new {
            if let Some(idx) = self.selected_entry_index() {
                let old_pw = &self.vault.entries[idx].password;
                if !old_pw.is_empty() && *old_pw != self.edit_buffer.password {
                    self.edit_buffer.password_history.push(PasswordHistoryItem {
                        password: old_pw.clone(),
                        changed_at: self.vault.entries[idx].modified_at,
                    });
                    // Keep max history
                    while self.edit_buffer.password_history.len() > MAX_PASSWORD_HISTORY {
                        self.edit_buffer.password_history.remove(0);
                    }
                }
            }
        }

        self.edit_buffer.modified_at = chrono::Utc::now().timestamp();

        // Duplicate detection
        let dup_warning = self.check_duplicate_on_save();

        if self.edit_is_new {
            self.vault.entries.push(self.edit_buffer.clone());
        } else if let Some(idx) = self.selected_entry_index() {
            self.vault.entries[idx] = self.edit_buffer.clone();
        }
        self.dirty = true;
        self.screen = Screen::List;
        self.input_mode = InputMode::Normal;
        self.update_filter();

        if let Some(warning) = dup_warning {
            self.set_status(format!("Saved (warning: {})", warning));
        } else {
            self.set_status("Entry saved");
        }
    }

    fn check_duplicate_on_save(&self) -> Option<String> {
        if self.edit_buffer.username.is_empty() || self.edit_buffer.url.is_empty() {
            return None;
        }
        let count = self
            .vault
            .entries
            .iter()
            .filter(|e| {
                e.id != self.edit_buffer.id
                    && !e.username.is_empty()
                    && !e.url.is_empty()
                    && e.username.to_lowercase() == self.edit_buffer.username.to_lowercase()
                    && e.url.to_lowercase() == self.edit_buffer.url.to_lowercase()
            })
            .count();
        if count > 0 {
            Some(format!("duplicate user+url found in {} other entries", count))
        } else {
            None
        }
    }

    pub fn delete_selected(&mut self) {
        if let Some(idx) = self.selected_entry_index() {
            self.vault.entries.remove(idx);
            self.dirty = true;
            self.update_filter();
            self.set_status("Entry deleted");
        }
        self.screen = Screen::List;
        self.input_mode = InputMode::Normal;
    }

    pub fn current_fields(&self) -> &'static [EntryField] {
        match self.edit_buffer.kind {
            EntryKind::Password => EntryField::all_password(),
            EntryKind::Note => EntryField::all_note(),
        }
    }

    pub fn current_field(&self) -> &EntryField {
        let fields = self.current_fields();
        &fields[self.active_field % fields.len()]
    }

    pub fn get_field_value(&self, field: &EntryField) -> &str {
        match field {
            EntryField::Folder => &self.edit_buffer.folder,
            EntryField::Name => &self.edit_buffer.name,
            EntryField::Username => &self.edit_buffer.username,
            EntryField::Password => &self.edit_buffer.password,
            EntryField::Url => &self.edit_buffer.url,
            EntryField::TotpSecret => &self.edit_buffer.totp_secret,
            EntryField::Tags => &self.edit_tags_buffer,
            EntryField::Notes => &self.edit_buffer.notes,
        }
    }

    pub fn get_field_value_mut(&mut self, field: &EntryField) -> &mut String {
        match field {
            EntryField::Folder => &mut self.edit_buffer.folder,
            EntryField::Name => &mut self.edit_buffer.name,
            EntryField::Username => &mut self.edit_buffer.username,
            EntryField::Password => &mut self.edit_buffer.password,
            EntryField::Url => &mut self.edit_buffer.url,
            EntryField::TotpSecret => &mut self.edit_buffer.totp_secret,
            EntryField::Tags => &mut self.edit_tags_buffer,
            EntryField::Notes => &mut self.edit_buffer.notes,
        }
    }

    pub fn field_push_char(&mut self, c: char) {
        let field = self.current_field().clone();
        self.get_field_value_mut(&field).push(c);
    }

    pub fn field_pop_char(&mut self) {
        let field = self.current_field().clone();
        self.get_field_value_mut(&field).pop();
    }

    pub fn next_field(&mut self) {
        let len = self.current_fields().len();
        self.active_field = (self.active_field + 1) % len;
    }

    pub fn prev_field(&mut self) {
        let len = self.current_fields().len();
        self.active_field = (self.active_field + len - 1) % len;
    }

    pub fn generate_password(&self) -> String {
        use rand::Rng;
        let mut charset = String::new();
        if self.gen_lowercase {
            charset.push_str("abcdefghijklmnopqrstuvwxyz");
        }
        if self.gen_uppercase {
            charset.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        }
        if self.gen_digits {
            charset.push_str("0123456789");
        }
        if self.gen_symbols {
            charset.push_str("!@#$%^&*()-_=+[]{}|;:,.<>?");
        }
        if charset.is_empty() {
            charset.push_str("abcdefghijklmnopqrstuvwxyz");
        }
        let chars: Vec<char> = charset.chars().collect();
        let mut rng = rand::thread_rng();
        (0..self.gen_length)
            .map(|_| chars[rng.gen_range(0..chars.len())])
            .collect()
    }

    pub fn navigate_into_folder(&mut self) {
        if let Some(ListRow::Folder(folder)) = self.list_rows.get(self.selected) {
            self.current_folder = folder.clone();
            self.selected = 0;
            self.update_filter();
        }
    }

    pub fn navigate_up_folder(&mut self) {
        if self.current_folder.is_empty() {
            return;
        }
        if let Some(pos) = self.current_folder.rfind('/') {
            self.current_folder = self.current_folder[..pos].to_string();
        } else {
            self.current_folder = String::new();
        }
        self.selected = 0;
        self.update_filter();
    }

    // ─── Tab & Settings ───────────────────────────────────────────

    pub fn switch_tab(&mut self) {
        match self.active_tab {
            Tab::Vault => {
                self.active_tab = Tab::Settings;
                self.screen = Screen::Settings;
                self.input_mode = InputMode::Normal;
            }
            Tab::Settings => {
                self.active_tab = Tab::Vault;
                self.screen = Screen::List;
                self.input_mode = InputMode::Normal;
            }
        }
    }

    pub fn reload_theme(&mut self) {
        self.theme = Theme::from_name(&self.config.theme);
    }

    pub fn start_password_change(&mut self) {
        self.pw_change_step = PasswordChangeStep::CurrentPassword;
        self.pw_change_current.clear();
        self.pw_change_new.clear();
        self.pw_change_confirm.clear();
        self.pw_change_error = None;
        self.screen = Screen::ChangePassword;
    }

    pub fn start_import(&mut self) {
        self.import_path_input.clear();
        self.screen = Screen::ImportPath;
    }

    // ─── Stats ────────────────────────────────────────────────────

    pub fn compute_stats(&self) -> VaultStats {
        let now = chrono::Utc::now().timestamp();
        let passwords: Vec<&Entry> = self
            .vault
            .entries
            .iter()
            .filter(|e| matches!(e.kind, EntryKind::Password))
            .collect();
        let notes_count = self
            .vault
            .entries
            .iter()
            .filter(|e| matches!(e.kind, EntryKind::Note))
            .count();

        let favorites = self.vault.entries.iter().filter(|e| e.favorite).count();

        // Unique folders
        let mut folders_set: Vec<String> = self
            .vault
            .entries
            .iter()
            .filter(|e| !e.folder.is_empty())
            .map(|e| e.folder.clone())
            .collect();
        folders_set.sort();
        folders_set.dedup();

        // Weak passwords
        let weak = passwords
            .iter()
            .filter(|e| {
                let (score, _, _) = crate::vault::password_strength_score(&e.password);
                score <= 1
            })
            .count();

        // Duplicate passwords
        let mut pw_counts: HashMap<String, usize> = HashMap::new();
        for e in &passwords {
            if !e.password.is_empty() {
                *pw_counts.entry(e.password.clone()).or_insert(0) += 1;
            }
        }
        let duplicate_passwords = pw_counts.values().filter(|&&c| c > 1).sum::<usize>();

        // Average age
        let total_age: u64 = self
            .vault
            .entries
            .iter()
            .map(|e| ((now - e.modified_at).max(0) as u64) / 86400)
            .sum();
        let avg_age = if self.vault.entries.is_empty() {
            0
        } else {
            total_age / self.vault.entries.len() as u64
        };

        let oldest = self
            .vault
            .entries
            .iter()
            .map(|e| ((now - e.created_at).max(0) as u64) / 86400)
            .max()
            .unwrap_or(0);

        // Tags count
        let mut all_tags: Vec<String> = self
            .vault
            .entries
            .iter()
            .flat_map(|e| e.tags.clone())
            .collect();
        all_tags.sort();
        all_tags.dedup();

        // TOTP count
        let totp_count = self
            .vault
            .entries
            .iter()
            .filter(|e| !e.totp_secret.is_empty())
            .count();

        VaultStats {
            total: self.vault.entries.len(),
            passwords: passwords.len(),
            notes: notes_count,
            favorites,
            folders: folders_set.len(),
            weak_passwords: weak,
            duplicate_passwords,
            avg_age_days: avg_age,
            oldest_days: oldest,
            tags_count: all_tags.len(),
            totp_count,
        }
    }
}

fn sort_indices(vault_entries: &[Entry], sort_mode: SortMode, entries: &mut [usize]) {
    entries.sort_by(|a, b| {
        let ea = &vault_entries[*a];
        let eb = &vault_entries[*b];
        eb.favorite.cmp(&ea.favorite).then_with(|| match sort_mode {
            SortMode::Name => ea.name.to_lowercase().cmp(&eb.name.to_lowercase()),
            SortMode::DateCreated => eb.created_at.cmp(&ea.created_at),
            SortMode::DateModified => eb.modified_at.cmp(&ea.modified_at),
        })
    });
}

fn top_folder(folder: &str) -> String {
    folder.split('/').next().unwrap_or("").to_string()
}
