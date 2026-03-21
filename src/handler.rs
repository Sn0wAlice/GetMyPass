use crate::app::{
    App, EntryField, InputMode, PasswordChangeStep, Screen, SettingsItem, Tab, SETTINGS_ITEMS,
};
use crate::clipboard::copy_to_clipboard;
use crate::config::save_config;
use crate::vault::{
    change_master_password, export_vault_json, import_vault_json, save_vault_with_backup,
    EntryKind,
};
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    app.touch_activity();
    app.clear_expired_status();

    if app.screen == Screen::Locked {
        handle_locked(app, key);
        return;
    }

    // Global tab switch: F1/F2 (except during editing/modals)
    if app.input_mode == InputMode::Normal
        && !matches!(
            app.screen,
            Screen::EditEntry
                | Screen::ConfirmDelete
                | Screen::GeneratePassword
                | Screen::ChangePassword
                | Screen::ImportPath
                | Screen::Stats
        )
    {
        match key.code {
            KeyCode::F(1) => {
                app.active_tab = Tab::Vault;
                app.screen = Screen::List;
                return;
            }
            KeyCode::F(2) => {
                app.active_tab = Tab::Settings;
                app.screen = Screen::Settings;
                return;
            }
            _ => {}
        }
    }

    match app.screen {
        Screen::List => handle_list(app, key),
        Screen::ViewEntry => handle_view(app, key),
        Screen::EditEntry => handle_edit(app, key),
        Screen::ConfirmDelete => handle_confirm_delete(app, key),
        Screen::GeneratePassword => handle_generate_password(app, key),
        Screen::Settings => handle_settings(app, key),
        Screen::ChangePassword => handle_change_password(app, key),
        Screen::ImportPath => handle_import_path(app, key),
        Screen::Stats => handle_stats(app, key),
        Screen::Locked => {}
    }
}

fn handle_locked(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            app.try_unlock();
        }
        KeyCode::Backspace => {
            app.lock_password_input.pop();
        }
        KeyCode::Char(c) => {
            app.lock_password_input.push(c);
        }
        KeyCode::Esc => {
            app.should_quit = true;
        }
        _ => {}
    }
}

fn handle_list(app: &mut App, key: KeyEvent) {
    match app.input_mode {
        InputMode::Normal => handle_list_normal(app, key),
        InputMode::Search => handle_list_search(app, key),
        _ => {}
    }
}

fn copy_password_with_clear(app: &mut App) {
    if let Some(entry) = app.selected_entry() {
        let pw = entry.password.clone();
        match copy_to_clipboard(&pw) {
            Ok(_) => {
                app.schedule_clipboard_clear();
                if app.config.clipboard_clear_seconds > 0 {
                    app.set_status(format!(
                        "Password copied (auto-clear in {})",
                        app.config.clipboard_clear_label()
                    ));
                } else {
                    app.set_status("Password copied to clipboard");
                }
            }
            Err(e) => app.set_status(e),
        }
    }
}

fn copy_username(app: &mut App) {
    if let Some(entry) = app.selected_entry() {
        let user = entry.username.clone();
        match copy_to_clipboard(&user) {
            Ok(_) => app.set_status("Username copied to clipboard"),
            Err(e) => app.set_status(e),
        }
    }
}

fn handle_list_normal(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => {
            app.switch_tab();
        }
        KeyCode::Char('q') => {
            if app.dirty {
                if let Err(e) = save_vault_with_backup(
                    &app.vault,
                    &app.master_password,
                    app.config.backup_enabled,
                ) {
                    app.set_status(format!("Save error: {}", e));
                    return;
                }
            }
            app.should_quit = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.list_rows.is_empty() {
                app.selected = (app.selected + 1) % app.list_rows.len();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.list_rows.is_empty() {
                app.selected =
                    (app.selected + app.list_rows.len() - 1) % app.list_rows.len();
            }
        }
        KeyCode::Char('/') | KeyCode::Char('s') => {
            app.input_mode = InputMode::Search;
            app.search_query.clear();
        }
        KeyCode::Enter => {
            if app.selected_is_folder() {
                app.toggle_folder_collapse();
            } else if app.selected_entry().is_some() {
                app.show_password = false;
                app.show_history = false;
                app.screen = Screen::ViewEntry;
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if app.selected_is_folder() {
                app.navigate_into_folder();
            } else if app.selected_entry().is_some() {
                app.show_password = false;
                app.show_history = false;
                app.screen = Screen::ViewEntry;
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.navigate_up_folder();
        }
        KeyCode::Backspace => {
            app.navigate_up_folder();
        }
        KeyCode::Char('n') => {
            app.start_new_entry(EntryKind::Password);
        }
        KeyCode::Char('N') => {
            app.start_new_entry(EntryKind::Note);
        }
        KeyCode::Char('e') => {
            if app.selected_entry().is_some() {
                app.show_password = true;
                app.start_edit_entry();
            }
        }
        KeyCode::Char('d') => {
            if app.selected_entry().is_some() {
                app.screen = Screen::ConfirmDelete;
            }
        }
        // Copy shortcuts: c/1 = password, u/2 = username
        KeyCode::Char('c') | KeyCode::Char('1') => {
            copy_password_with_clear(app);
        }
        KeyCode::Char('u') | KeyCode::Char('2') => {
            copy_username(app);
        }
        // Favorite toggle
        KeyCode::Char('f') => {
            app.toggle_favorite();
        }
        // Duplicate entry
        KeyCode::Char('D') => {
            app.duplicate_selected();
        }
        // Sort cycle
        KeyCode::Char('o') => {
            app.cycle_sort();
        }
        // Stats
        KeyCode::Char('i') => {
            app.screen = Screen::Stats;
        }
        _ => {}
    }
}

fn handle_list_search(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.search_query.clear();
            app.input_mode = InputMode::Normal;
            app.update_filter();
        }
        KeyCode::Enter => {
            app.input_mode = InputMode::Normal;
            if app.filtered_indices.len() == 1 {
                app.selected = 0;
                for (i, row) in app.list_rows.iter().enumerate() {
                    if matches!(row, crate::app::ListRow::Entry(_)) {
                        app.selected = i;
                        break;
                    }
                }
                app.show_password = false;
                app.show_history = false;
                app.screen = Screen::ViewEntry;
            }
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.update_filter();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.update_filter();
        }
        KeyCode::Down => {
            if !app.list_rows.is_empty() {
                app.selected = (app.selected + 1) % app.list_rows.len();
            }
        }
        KeyCode::Up => {
            if !app.list_rows.is_empty() {
                app.selected =
                    (app.selected + app.list_rows.len() - 1) % app.list_rows.len();
            }
        }
        _ => {}
    }
}

fn handle_view(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen = Screen::List;
        }
        KeyCode::Char('c') | KeyCode::Char('1') => {
            copy_password_with_clear(app);
        }
        KeyCode::Char('u') | KeyCode::Char('2') => {
            copy_username(app);
        }
        KeyCode::Char('p') => {
            app.show_password = !app.show_password;
        }
        KeyCode::Char('e') => {
            app.show_password = true;
            app.start_edit_entry();
        }
        KeyCode::Char('f') => {
            app.toggle_favorite();
        }
        KeyCode::Char('H') => {
            app.show_history = !app.show_history;
        }
        _ => {}
    }
}

fn handle_edit(app: &mut App, key: KeyEvent) {
    // F5 to save (SSH-safe, replaces Ctrl+S)
    if key.code == KeyCode::F(5) {
        app.save_edit();
        if app.dirty {
            if let Err(e) = save_vault_with_backup(
                &app.vault,
                &app.master_password,
                app.config.backup_enabled,
            ) {
                app.set_status(format!("Save error: {}", e));
            }
        }
        return;
    }

    // F6 to generate password (SSH-safe, replaces Ctrl+G)
    if key.code == KeyCode::F(6) {
        if matches!(app.edit_buffer.kind, EntryKind::Password) {
            app.gen_preview = app.generate_password();
            app.screen = Screen::GeneratePassword;
        }
        return;
    }

    match key.code {
        KeyCode::Esc => {
            app.screen = Screen::List;
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Tab => {
            app.next_field();
        }
        KeyCode::BackTab => {
            app.prev_field();
        }
        KeyCode::Backspace => {
            app.field_pop_char();
        }
        KeyCode::Enter => {
            if *app.current_field() == EntryField::Notes {
                let field = app.current_field().clone();
                app.get_field_value_mut(&field).push('\n');
            } else {
                app.next_field();
            }
        }
        KeyCode::Char(c) => {
            app.field_push_char(c);
        }
        _ => {}
    }
}

fn handle_confirm_delete(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.delete_selected();
            if app.dirty {
                if let Err(e) = save_vault_with_backup(
                    &app.vault,
                    &app.master_password,
                    app.config.backup_enabled,
                ) {
                    app.set_status(format!("Save error: {}", e));
                }
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.screen = Screen::List;
        }
        _ => {}
    }
}

fn handle_generate_password(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.screen = Screen::EditEntry;
        }
        KeyCode::Enter => {
            app.edit_buffer.password = app.gen_preview.clone();
            app.screen = Screen::EditEntry;
            app.set_status("Password generated");
        }
        KeyCode::Char('r') => {
            app.gen_preview = app.generate_password();
        }
        KeyCode::Char('1') => {
            app.gen_uppercase = !app.gen_uppercase;
            app.gen_preview = app.generate_password();
        }
        KeyCode::Char('2') => {
            app.gen_lowercase = !app.gen_lowercase;
            app.gen_preview = app.generate_password();
        }
        KeyCode::Char('3') => {
            app.gen_digits = !app.gen_digits;
            app.gen_preview = app.generate_password();
        }
        KeyCode::Char('4') => {
            app.gen_symbols = !app.gen_symbols;
            app.gen_preview = app.generate_password();
        }
        KeyCode::Left => {
            if app.gen_length > 4 {
                app.gen_length -= 1;
                app.gen_preview = app.generate_password();
            }
        }
        KeyCode::Right => {
            if app.gen_length < 128 {
                app.gen_length += 1;
                app.gen_preview = app.generate_password();
            }
        }
        _ => {}
    }
}

// ─── Stats ────────────────────────────────────────────────────────────

fn handle_stats(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('i') | KeyCode::Enter => {
            app.screen = Screen::List;
        }
        _ => {}
    }
}

// ─── Settings ─────────────────────────────────────────────────────────

fn handle_settings(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Tab | KeyCode::Esc => {
            app.switch_tab();
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.settings_selected = (app.settings_selected + 1) % SETTINGS_ITEMS.len();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.settings_selected = (app.settings_selected + SETTINGS_ITEMS.len() - 1)
                % SETTINGS_ITEMS.len();
        }
        KeyCode::Left => {
            handle_settings_adjust(app, false);
        }
        KeyCode::Right => {
            handle_settings_adjust(app, true);
        }
        KeyCode::Enter => {
            handle_settings_action(app);
        }
        _ => {}
    }
}

fn handle_settings_adjust(app: &mut App, forward: bool) {
    let item = SETTINGS_ITEMS[app.settings_selected];
    match item {
        SettingsItem::AutoLock => {
            app.config.cycle_auto_lock(forward);
            let _ = save_config(&app.config);
            app.set_status(format!("Auto-lock: {}", app.config.auto_lock_label()));
        }
        SettingsItem::ClipboardClear => {
            app.config.cycle_clipboard_clear(forward);
            let _ = save_config(&app.config);
            app.set_status(format!(
                "Clipboard clear: {}",
                app.config.clipboard_clear_label()
            ));
        }
        SettingsItem::AutoBackup => {
            app.config.backup_enabled = !app.config.backup_enabled;
            let _ = save_config(&app.config);
            app.set_status(format!(
                "Auto-backup: {}",
                if app.config.backup_enabled { "On" } else { "Off" }
            ));
        }
        SettingsItem::ThemeSetting => {
            app.config.cycle_theme();
            app.reload_theme();
            let _ = save_config(&app.config);
            app.set_status(format!("Theme: {}", app.config.theme));
        }
        SettingsItem::DefaultGenLength => {
            app.config.adjust_gen_length(forward);
            let _ = save_config(&app.config);
            app.set_status(format!(
                "Default password length: {}",
                app.config.default_gen_length
            ));
        }
        _ => {}
    }
}

fn handle_settings_action(app: &mut App) {
    let item = SETTINGS_ITEMS[app.settings_selected];
    match item {
        SettingsItem::AutoLock => handle_settings_adjust(app, true),
        SettingsItem::ClipboardClear => handle_settings_adjust(app, true),
        SettingsItem::AutoBackup => {
            app.config.backup_enabled = !app.config.backup_enabled;
            let _ = save_config(&app.config);
            app.set_status(format!(
                "Auto-backup: {}",
                if app.config.backup_enabled { "On" } else { "Off" }
            ));
        }
        SettingsItem::ThemeSetting => handle_settings_adjust(app, true),
        SettingsItem::DefaultGenLength => handle_settings_adjust(app, true),
        SettingsItem::ChangePassword => {
            app.start_password_change();
        }
        SettingsItem::ExportJson => match export_vault_json(&app.vault) {
            Ok(path) => app.set_status(format!("Exported to {}", path)),
            Err(e) => app.set_status(format!("Export failed: {}", e)),
        },
        SettingsItem::ImportJson => {
            app.start_import();
        }
    }
}

// ─── Password Change ──────────────────────────────────────────────────

fn handle_change_password(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.screen = Screen::Settings;
            app.pw_change_error = None;
        }
        KeyCode::Backspace => {
            match app.pw_change_step {
                PasswordChangeStep::CurrentPassword => {
                    app.pw_change_current.pop();
                }
                PasswordChangeStep::NewPassword => {
                    app.pw_change_new.pop();
                }
                PasswordChangeStep::ConfirmPassword => {
                    app.pw_change_confirm.pop();
                }
            }
            app.pw_change_error = None;
        }
        KeyCode::Enter => match app.pw_change_step {
            PasswordChangeStep::CurrentPassword => {
                if app.pw_change_current != app.master_password {
                    app.pw_change_error = Some("Wrong current password".to_string());
                    app.pw_change_current.clear();
                } else {
                    app.pw_change_error = None;
                    app.pw_change_step = PasswordChangeStep::NewPassword;
                }
            }
            PasswordChangeStep::NewPassword => {
                if app.pw_change_new.len() < 8 {
                    app.pw_change_error =
                        Some("New password must be at least 8 characters".to_string());
                } else {
                    app.pw_change_error = None;
                    app.pw_change_step = PasswordChangeStep::ConfirmPassword;
                }
            }
            PasswordChangeStep::ConfirmPassword => {
                if app.pw_change_new != app.pw_change_confirm {
                    app.pw_change_error = Some("Passwords do not match".to_string());
                    app.pw_change_confirm.clear();
                } else {
                    match change_master_password(&app.vault, &app.pw_change_new) {
                        Ok(_) => {
                            app.master_password = app.pw_change_new.clone();
                            app.pw_change_current.clear();
                            app.pw_change_new.clear();
                            app.pw_change_confirm.clear();
                            app.pw_change_error = None;
                            app.screen = Screen::Settings;
                            app.set_status("Master password changed successfully");
                        }
                        Err(e) => {
                            app.pw_change_error = Some(format!("Failed: {}", e));
                        }
                    }
                }
            }
        },
        KeyCode::Char(c) => {
            match app.pw_change_step {
                PasswordChangeStep::CurrentPassword => app.pw_change_current.push(c),
                PasswordChangeStep::NewPassword => app.pw_change_new.push(c),
                PasswordChangeStep::ConfirmPassword => app.pw_change_confirm.push(c),
            }
            app.pw_change_error = None;
        }
        _ => {}
    }
}

// ─── Import ───────────────────────────────────────────────────────────

fn handle_import_path(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.screen = Screen::Settings;
        }
        KeyCode::Backspace => {
            app.import_path_input.pop();
        }
        KeyCode::Enter => {
            let path = app.import_path_input.trim().to_string();
            if path.is_empty() {
                app.set_status("No path entered");
                app.screen = Screen::Settings;
                return;
            }
            let expanded = if path.starts_with("~/") {
                if let Some(home) = dirs::home_dir() {
                    home.join(&path[2..]).to_string_lossy().to_string()
                } else {
                    path.clone()
                }
            } else {
                path.clone()
            };

            match import_vault_json(&mut app.vault, &expanded) {
                Ok(count) => {
                    app.dirty = true;
                    if let Err(e) = save_vault_with_backup(
                        &app.vault,
                        &app.master_password,
                        app.config.backup_enabled,
                    ) {
                        app.set_status(format!("Import OK but save error: {}", e));
                    } else {
                        app.set_status(format!("Imported {} entries", count));
                    }
                    app.update_filter();
                    app.screen = Screen::Settings;
                }
                Err(e) => {
                    app.set_status(format!("Import error: {}", e));
                    app.screen = Screen::Settings;
                }
            }
        }
        KeyCode::Char(c) => {
            app.import_path_input.push(c);
        }
        _ => {}
    }
}
