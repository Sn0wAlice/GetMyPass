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
    Name,
    Username,
    Password,
    Url,
    Notes,
}

impl EntryField {
    pub fn all_password() -> &'static [EntryField] {
        &[
            EntryField::Name,
            EntryField::Username,
            EntryField::Password,
            EntryField::Url,
            EntryField::Notes,
        ]
    }

    pub fn all_note() -> &'static [EntryField] {
        &[EntryField::Name, EntryField::Notes]
    }

    pub fn label(&self) -> &str {
        match self {
            EntryField::Name => "Name",
            EntryField::Username => "Username",
            EntryField::Password => "Password",
            EntryField::Url => "URL",
            EntryField::Notes => "Notes",
        }
    }
}

pub struct App {
    pub vault: Vault,
    pub master_password: String,
    pub screen: Screen,
    pub input_mode: InputMode,
    pub search_query: String,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub edit_buffer: Entry,
    pub edit_is_new: bool,
    pub active_field: usize,
    pub show_password: bool,
    pub status_message: Option<(String, Instant)>,
    pub should_quit: bool,
    pub dirty: bool,
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
            selected: 0,
            edit_buffer: Entry::new_password(),
            edit_is_new: true,
            active_field: 0,
            show_password: false,
            status_message: None,
            should_quit: false,
            dirty: false,
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
        if self.search_query.is_empty() {
            self.filtered_indices = (0..self.vault.entries.len()).collect();
        } else {
            self.filtered_indices = self
                .vault
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| e.matches(&self.search_query))
                .map(|(i, _)| i)
                .collect();
        }
        if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len().saturating_sub(1);
        }
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&i| self.vault.entries.get(i))
    }

    pub fn selected_entry_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.selected).copied()
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
            EntryField::Name => &self.edit_buffer.name,
            EntryField::Username => &self.edit_buffer.username,
            EntryField::Password => &self.edit_buffer.password,
            EntryField::Url => &self.edit_buffer.url,
            EntryField::Notes => &self.edit_buffer.notes,
        }
    }

    pub fn get_field_value_mut(&mut self, field: &EntryField) -> &mut String {
        match field {
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
}
