use crate::app::{App, EntryField, InputMode, Screen};
use crate::clipboard::copy_to_clipboard;
use crate::vault::{save_vault, EntryKind};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    app.clear_expired_status();

    match app.screen {
        Screen::List => handle_list(app, key),
        Screen::ViewEntry => handle_view(app, key),
        Screen::EditEntry => handle_edit(app, key),
        Screen::ConfirmDelete => handle_confirm_delete(app, key),
        Screen::GeneratePassword => handle_generate_password(app, key),
    }
}

fn handle_list(app: &mut App, key: KeyEvent) {
    match app.input_mode {
        InputMode::Normal => handle_list_normal(app, key),
        InputMode::Search => handle_list_search(app, key),
        _ => {}
    }
}

fn handle_list_normal(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => {
            if app.dirty {
                if let Err(e) = save_vault(&app.vault, &app.master_password) {
                    app.set_status(format!("Save error: {}", e));
                    return;
                }
            }
            app.should_quit = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.filtered_indices.is_empty() {
                app.selected = (app.selected + 1) % app.filtered_indices.len();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.filtered_indices.is_empty() {
                app.selected = (app.selected + app.filtered_indices.len() - 1)
                    % app.filtered_indices.len();
            }
        }
        KeyCode::Char('/') | KeyCode::Char('s') => {
            app.input_mode = InputMode::Search;
            app.search_query.clear();
        }
        KeyCode::Enter => {
            if app.selected_entry().is_some() {
                app.show_password = false;
                app.screen = Screen::ViewEntry;
            }
        }
        KeyCode::Char('n') => {
            app.start_new_entry(EntryKind::Password);
        }
        KeyCode::Char('N') => {
            app.start_new_entry(EntryKind::Note);
        }
        KeyCode::Char('e') => {
            app.show_password = true;
            app.start_edit_entry();
        }
        KeyCode::Char('d') => {
            if app.selected_entry().is_some() {
                app.screen = Screen::ConfirmDelete;
            }
        }
        KeyCode::Char('c') => {
            if let Some(entry) = app.selected_entry() {
                let pw = entry.password.clone();
                match copy_to_clipboard(&pw) {
                    Ok(_) => app.set_status("Password copied to clipboard"),
                    Err(e) => app.set_status(e),
                }
            }
        }
        KeyCode::Char('u') => {
            if let Some(entry) = app.selected_entry() {
                let user = entry.username.clone();
                match copy_to_clipboard(&user) {
                    Ok(_) => app.set_status("Username copied to clipboard"),
                    Err(e) => app.set_status(e),
                }
            }
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
                app.show_password = false;
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
            if !app.filtered_indices.is_empty() {
                app.selected = (app.selected + 1) % app.filtered_indices.len();
            }
        }
        KeyCode::Up => {
            if !app.filtered_indices.is_empty() {
                app.selected = (app.selected + app.filtered_indices.len() - 1)
                    % app.filtered_indices.len();
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
        KeyCode::Char('c') => {
            if let Some(entry) = app.selected_entry() {
                let pw = entry.password.clone();
                match copy_to_clipboard(&pw) {
                    Ok(_) => app.set_status("Password copied to clipboard"),
                    Err(e) => app.set_status(e),
                }
            }
        }
        KeyCode::Char('u') => {
            if let Some(entry) = app.selected_entry() {
                let user = entry.username.clone();
                match copy_to_clipboard(&user) {
                    Ok(_) => app.set_status("Username copied to clipboard"),
                    Err(e) => app.set_status(e),
                }
            }
        }
        KeyCode::Char('p') => {
            app.show_password = !app.show_password;
        }
        KeyCode::Char('e') => {
            app.show_password = true;
            app.start_edit_entry();
        }
        _ => {}
    }
}

fn handle_edit(app: &mut App, key: KeyEvent) {
    // Ctrl+S to save
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        app.save_edit();
        if app.dirty {
            if let Err(e) = save_vault(&app.vault, &app.master_password) {
                app.set_status(format!("Save error: {}", e));
            }
        }
        return;
    }

    // Ctrl+G to generate password
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('g') {
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
            // In notes field, add newline
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
                if let Err(e) = save_vault(&app.vault, &app.master_password) {
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
