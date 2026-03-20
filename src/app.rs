use crate::vault::{Entry, EntryKind, Vault};
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    List,
    ViewEntry,
    EditEntry,
    ConfirmDelete,
    GeneratePassword,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
    Editing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntryField {
    Folder,
    Name,
    Username,
    Password,
    Url,
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
            EntryField::Notes,
        ]
    }

    pub fn all_note() -> &'static [EntryField] {
        &[EntryField::Folder, EntryField::Name, EntryField::Notes]
    }

    pub fn label(&self) -> &str {
        match self {
            EntryField::Folder => "Folder",
            EntryField::Name => "Name",
            EntryField::Username => "Username",
            EntryField::Password => "Password",
            EntryField::Url => "URL",
            EntryField::Notes => "Notes",
        }
    }
}

/// Represents a row in the list view — either a folder header or an entry
#[derive(Debug, Clone)]
pub enum ListRow {
    Folder(String),       // folder path
    Entry(usize),         // index into vault.entries
}

pub struct App {
    pub vault: Vault,
    pub master_password: String,
    pub screen: Screen,
    pub input_mode: InputMode,
    pub search_query: String,
    pub filtered_indices: Vec<usize>,
    pub list_rows: Vec<ListRow>,
    pub selected: usize,
    pub edit_buffer: Entry,
    pub edit_is_new: bool,
    pub active_field: usize,
    pub show_password: bool,
    pub status_message: Option<(String, Instant)>,
    pub should_quit: bool,
    pub dirty: bool,
    // Folder navigation
    pub current_folder: String, // "" = root, "Work" = Work folder, "Work/Email" = subfolder
    pub collapsed_folders: Vec<String>,
    // Password generator state
    pub gen_length: usize,
    pub gen_uppercase: bool,
    pub gen_lowercase: bool,
    pub gen_digits: bool,
    pub gen_symbols: bool,
    pub gen_preview: String,
}

impl App {
    pub fn new(vault: Vault, master_password: String) -> Self {
        let mut app = Self {
            vault,
            master_password,
            screen: Screen::List,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            filtered_indices: Vec::new(),
            list_rows: Vec::new(),
            selected: 0,
            edit_buffer: Entry::new_password(),
            edit_is_new: true,
            active_field: 0,
            show_password: false,
            status_message: None,
            should_quit: false,
            dirty: false,
            current_folder: String::new(),
            collapsed_folders: Vec::new(),
            gen_length: 20,
            gen_uppercase: true,
            gen_lowercase: true,
            gen_digits: true,
            gen_symbols: true,
            gen_preview: String::new(),
        };
        app.update_filter();
        app
    }

    pub fn update_filter(&mut self) {
        let is_searching = !self.search_query.is_empty();

        if is_searching {
            // In search mode: flat list, no folders
            self.filtered_indices = self
                .vault
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| e.matches(&self.search_query))
                .map(|(i, _)| i)
                .collect();
            self.list_rows = self
                .filtered_indices
                .iter()
                .map(|&i| ListRow::Entry(i))
                .collect();
        } else {
            // Folder-based view
            self.build_folder_rows();
        }

        if self.selected >= self.list_rows.len() {
            self.selected = self.list_rows.len().saturating_sub(1);
        }
    }

    fn build_folder_rows(&mut self) {
        let mut rows: Vec<ListRow> = Vec::new();
        let mut seen_folders: Vec<String> = Vec::new();

        // Collect entries matching current folder view
        // Sort entries: folders first (alphabetically), then entries alphabetically
        let mut folder_entries: Vec<(String, usize)> = Vec::new();
        let mut direct_subfolders: Vec<String> = Vec::new();

        for (i, entry) in self.vault.entries.iter().enumerate() {
            if self.current_folder.is_empty() {
                // Root level: show entries with no folder and first-level folder headers
                if entry.folder.is_empty() {
                    folder_entries.push((String::new(), i));
                } else {
                    let top = entry.folder.split('/').next().unwrap_or("").to_string();
                    if !direct_subfolders.contains(&top) {
                        direct_subfolders.push(top);
                    }
                    // Show entries if folder is not collapsed
                    if !self.is_folder_collapsed(&top_folder(&entry.folder)) {
                        folder_entries.push((entry.folder.clone(), i));
                    }
                }
            } else {
                // Inside a folder: show entries in this folder and subfolders
                if entry.folder == self.current_folder {
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
        }

        direct_subfolders.sort();

        // Add subfolder headers and their entries
        for subfolder in &direct_subfolders {
            rows.push(ListRow::Folder(subfolder.clone()));
            seen_folders.push(subfolder.clone());

            if !self.is_folder_collapsed(subfolder) {
                // Add entries in this subfolder (and nested)
                let mut sub_entries: Vec<usize> = folder_entries
                    .iter()
                    .filter(|(f, _)| f == subfolder || f.starts_with(&format!("{}/", subfolder)))
                    .map(|(_, i)| *i)
                    .collect();
                sub_entries.sort_by(|a, b| {
                    self.vault.entries[*a]
                        .name
                        .to_lowercase()
                        .cmp(&self.vault.entries[*b].name.to_lowercase())
                });
                for idx in sub_entries {
                    rows.push(ListRow::Entry(idx));
                }
            }
        }

        // Add root-level entries (no folder)
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
        root_entries.sort_by(|a, b| {
            self.vault.entries[*a]
                .name
                .to_lowercase()
                .cmp(&self.vault.entries[*b].name.to_lowercase())
        });
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

    pub fn start_new_entry(&mut self, kind: EntryKind) {
        self.edit_buffer = match kind {
            EntryKind::Password => Entry::new_password(),
            EntryKind::Note => Entry::new_note(),
        };
        // Pre-fill folder with current folder
        self.edit_buffer.folder = self.current_folder.clone();
        self.edit_is_new = true;
        self.active_field = 0;
        self.screen = Screen::EditEntry;
        self.input_mode = InputMode::Editing;
    }

    pub fn start_edit_entry(&mut self) {
        if let Some(entry) = self.selected_entry() {
            self.edit_buffer = entry.clone();
            self.edit_is_new = false;
            self.active_field = 0;
            self.screen = Screen::EditEntry;
            self.input_mode = InputMode::Editing;
        }
    }

    pub fn save_edit(&mut self) {
        // Validate folder depth (max 3 levels)
        let depth = if self.edit_buffer.folder.is_empty() {
            0
        } else {
            self.edit_buffer.folder.matches('/').count() + 1
        };
        if depth > 3 {
            self.set_status("Folder depth limited to 3 levels");
            return;
        }

        // Clean up folder path (remove trailing/leading slashes)
        self.edit_buffer.folder = self
            .edit_buffer
            .folder
            .trim_matches('/')
            .to_string();

        self.edit_buffer.modified_at = chrono::Utc::now().timestamp();
        if self.edit_is_new {
            self.vault.entries.push(self.edit_buffer.clone());
        } else if let Some(idx) = self.selected_entry_index() {
            self.vault.entries[idx] = self.edit_buffer.clone();
        }
        self.dirty = true;
        self.screen = Screen::List;
        self.input_mode = InputMode::Normal;
        self.update_filter();
        self.set_status("Entry saved");
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
}

fn top_folder(folder: &str) -> String {
    folder.split('/').next().unwrap_or("").to_string()
}
