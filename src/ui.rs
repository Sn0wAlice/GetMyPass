use crate::app::{App, EntryField, InputMode, Screen};
use crate::vault::EntryKind;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    match app.screen {
        Screen::List => draw_list(f, app),
        Screen::ViewEntry => draw_view(f, app),
        Screen::EditEntry => draw_edit(f, app),
        Screen::ConfirmDelete => {
            draw_list(f, app);
            draw_confirm_delete(f, app);
        }
        Screen::GeneratePassword => {
            draw_edit(f, app);
            draw_generate_password(f, app);
        }
    }
}

fn draw_list(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title + search
            Constraint::Min(5),   // list
            Constraint::Length(3), // help bar
        ])
        .split(f.area());

    // Search bar
    let search_style = if app.input_mode == InputMode::Search {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let search_text = if app.search_query.is_empty() && app.input_mode != InputMode::Search {
        "Press / to search...".to_string()
    } else {
        app.search_query.clone()
    };
    let search = Paragraph::new(search_text)
        .style(search_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" GetMyPass - Search ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        );
    f.render_widget(search, chunks[0]);

    if app.input_mode == InputMode::Search {
        f.set_cursor_position((
            chunks[0].x + app.search_query.len() as u16 + 1,
            chunks[0].y + 1,
        ));
    }

    // Entry list
    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .map(|&i| {
            let entry = &app.vault.entries[i];
            let icon = match entry.kind {
                EntryKind::Password => "🔑",
                EntryKind::Note => "📝",
            };
            let detail = match entry.kind {
                EntryKind::Password => {
                    if entry.username.is_empty() {
                        entry.url.clone()
                    } else {
                        format!("{} · {}", entry.username, entry.url)
                    }
                }
                EntryKind::Note => {
                    let preview: String = entry.notes.chars().take(40).collect();
                    preview.replace('\n', " ")
                }
            };
            let line = Line::from(vec![
                Span::raw(format!(" {} ", icon)),
                Span::styled(
                    format!("{:<20}", truncate_str(&entry.name, 20)),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    truncate_str(&detail, 40),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let count = app.filtered_indices.len();
    let total = app.vault.entries.len();
    let list_title = if app.search_query.is_empty() {
        format!(" Entries ({}) ", total)
    } else {
        format!(" Results ({}/{}) ", count, total)
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(list_title),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    if !app.filtered_indices.is_empty() {
        state.select(Some(app.selected));
    }
    f.render_stateful_widget(list, chunks[1], &mut state);

    // Help bar
    let help_text = match &app.status_message {
        Some((msg, _)) => Line::from(Span::styled(
            msg.as_str(),
            Style::default().fg(Color::Green),
        )),
        None => Line::from(vec![
            Span::styled(" n", Style::default().fg(Color::Yellow)),
            Span::raw(" new  "),
            Span::styled("N", Style::default().fg(Color::Yellow)),
            Span::raw(" note  "),
            Span::styled("e", Style::default().fg(Color::Yellow)),
            Span::raw(" edit  "),
            Span::styled("d", Style::default().fg(Color::Yellow)),
            Span::raw(" del  "),
            Span::styled("c", Style::default().fg(Color::Yellow)),
            Span::raw(" copy pw  "),
            Span::styled("u", Style::default().fg(Color::Yellow)),
            Span::raw(" copy user  "),
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(" search  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" quit"),
        ]),
    };
    let help = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Help "),
    );
    f.render_widget(help, chunks[2]);
}

fn draw_view(f: &mut Frame, app: &App) {
    let entry = match app.selected_entry() {
        Some(e) => e,
        None => return,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area());

    let icon = match entry.kind {
        EntryKind::Password => "🔑",
        EntryKind::Note => "📝",
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Name:     ", Style::default().fg(Color::Cyan)),
            Span::raw(&entry.name),
        ]),
    ];

    match entry.kind {
        EntryKind::Password => {
            lines.push(Line::from(vec![
                Span::styled("  Username: ", Style::default().fg(Color::Cyan)),
                Span::raw(&entry.username),
            ]));
            let pw_display = if app.show_password {
                entry.password.clone()
            } else {
                "••••••••••••".to_string()
            };
            lines.push(Line::from(vec![
                Span::styled("  Password: ", Style::default().fg(Color::Cyan)),
                Span::raw(pw_display),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  URL:      ", Style::default().fg(Color::Cyan)),
                Span::raw(&entry.url),
            ]));
        }
        EntryKind::Note => {}
    }

    if !entry.notes.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Notes:",
            Style::default().fg(Color::Cyan),
        )));
        for note_line in entry.notes.lines() {
            lines.push(Line::from(format!("  {}", note_line)));
        }
    }

    lines.push(Line::from(""));
    let modified = chrono::DateTime::from_timestamp(entry.modified_at, 0)
        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default();
    lines.push(Line::from(vec![
        Span::styled("  Modified: ", Style::default().fg(Color::DarkGray)),
        Span::styled(modified, Style::default().fg(Color::DarkGray)),
    ]));

    let title = format!(" {} {} ", icon, entry.name);
    let view = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(view, chunks[0]);

    // Help
    let help_text = match &app.status_message {
        Some((msg, _)) => Line::from(Span::styled(msg.as_str(), Style::default().fg(Color::Green))),
        None => Line::from(vec![
            Span::styled(" c", Style::default().fg(Color::Yellow)),
            Span::raw(" copy pw  "),
            Span::styled("u", Style::default().fg(Color::Yellow)),
            Span::raw(" copy user  "),
            Span::styled("p", Style::default().fg(Color::Yellow)),
            Span::raw(" show/hide pw  "),
            Span::styled("e", Style::default().fg(Color::Yellow)),
            Span::raw(" edit  "),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" back"),
        ]),
    };
    let help = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Help "),
    );
    f.render_widget(help, chunks[1]);
}

fn draw_edit(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area());

    let fields = app.current_fields();
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for (i, field) in fields.iter().enumerate() {
        let is_active = i == app.active_field;
        let label_style = if is_active {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let value = app.get_field_value(field);
        let display_value = if *field == EntryField::Password && !app.show_password {
            "•".repeat(value.len())
        } else {
            value.to_string()
        };

        let indicator = if is_active { "▶ " } else { "  " };

        if *field == EntryField::Notes {
            lines.push(Line::from(vec![
                Span::raw(indicator),
                Span::styled(format!("{:<10}", field.label()), label_style),
            ]));
            for note_line in display_value.lines() {
                lines.push(Line::from(format!("    {}", note_line)));
            }
            if display_value.is_empty() || display_value.ends_with('\n') {
                lines.push(Line::from("    "));
            }
        } else {
            lines.push(Line::from(vec![
                Span::raw(indicator),
                Span::styled(format!("{:<10} ", field.label()), label_style),
                Span::raw(display_value),
            ]));
        }
    }

    let kind_str = match app.edit_buffer.kind {
        EntryKind::Password => "Password",
        EntryKind::Note => "Note",
    };
    let action = if app.edit_is_new { "New" } else { "Edit" };
    let title = format!(" {} {} ", action, kind_str);

    let edit = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(edit, chunks[0]);

    // Help
    let help_text = Line::from(vec![
        Span::styled(" Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" next  "),
        Span::styled("S-Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" prev  "),
        Span::styled("Ctrl+S", Style::default().fg(Color::Yellow)),
        Span::raw(" save  "),
        Span::styled("Ctrl+G", Style::default().fg(Color::Yellow)),
        Span::raw(" gen pw  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" cancel"),
    ]);
    let help = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Help "),
    );
    f.render_widget(help, chunks[1]);
}

fn draw_confirm_delete(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 30, f.area());
    f.render_widget(Clear, area);

    let name = app
        .selected_entry()
        .map(|e| e.name.as_str())
        .unwrap_or("?");

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Delete \"{}\"?", name),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  This action cannot be undone."),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" yes  "),
            Span::styled("n", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" no"),
        ]),
    ];

    let popup = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Confirm Delete ")
            .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
    );
    f.render_widget(popup, area);
}

fn draw_generate_password(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 50, f.area());
    f.render_widget(Clear, area);

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Length: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{}", app.gen_length),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("  (←/→ to adjust)", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            checkbox(app.gen_uppercase),
            Span::styled(" 1", Style::default().fg(Color::Yellow)),
            Span::raw(" Uppercase (A-Z)"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            checkbox(app.gen_lowercase),
            Span::styled(" 2", Style::default().fg(Color::Yellow)),
            Span::raw(" Lowercase (a-z)"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            checkbox(app.gen_digits),
            Span::styled(" 3", Style::default().fg(Color::Yellow)),
            Span::raw(" Digits (0-9)"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            checkbox(app.gen_symbols),
            Span::styled(" 4", Style::default().fg(Color::Yellow)),
            Span::raw(" Symbols (!@#$...)"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Preview: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                &app.gen_preview,
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" accept  "),
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::raw(" regenerate  "),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" cancel"),
        ]),
    ];

    let popup = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Generate Password ")
            .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    );
    f.render_widget(popup, area);
}

fn checkbox(checked: bool) -> Span<'static> {
    if checked {
        Span::styled("[x]", Style::default().fg(Color::Green))
    } else {
        Span::styled("[ ]", Style::default().fg(Color::DarkGray))
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
