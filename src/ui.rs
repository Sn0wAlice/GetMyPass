use crate::app::{App, EntryField, InputMode, ListRow, Screen};
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
    let area = f.area();
    let width = area.width as usize;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // search
            Constraint::Min(5),   // list
            Constraint::Length(3), // help bar
        ])
        .split(area);

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

    // Show breadcrumb in title
    let title = if app.current_folder.is_empty() {
        " GetMyPass - Search ".to_string()
    } else {
        format!(" GetMyPass - /{} - Search ", app.current_folder)
    };

    let search = Paragraph::new(search_text)
        .style(search_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        );
    f.render_widget(search, chunks[0]);

    if app.input_mode == InputMode::Search {
        f.set_cursor_position((
            chunks[0].x + app.search_query.len() as u16 + 1,
            chunks[0].y + 1,
        ));
    }

    // Dynamic column widths based on terminal width
    let usable = width.saturating_sub(8); // icon + padding + highlight symbol
    let name_width = (usable * 35 / 100).max(15).min(50);
    let detail_width = usable.saturating_sub(name_width);

    // Entry list
    let items: Vec<ListItem> = app
        .list_rows
        .iter()
        .map(|row| match row {
            ListRow::Folder(folder) => {
                let display_name = folder.split('/').last().unwrap_or(folder);
                let is_collapsed = app.collapsed_folders.iter().any(|f| f == folder);
                let arrow = if is_collapsed { "▸" } else { "▾" };
                let depth = folder.matches('/').count();
                let indent = "  ".repeat(depth);
                let entry_count = app
                    .vault
                    .entries
                    .iter()
                    .filter(|e| e.folder == *folder || e.folder.starts_with(&format!("{}/", folder)))
                    .count();
                let line = Line::from(vec![
                    Span::raw(format!(" {} ", indent)),
                    Span::styled(
                        format!("{} 📁 {}", arrow, display_name),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  ({})", entry_count),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]);
                ListItem::new(line)
            }
            ListRow::Entry(i) => {
                let entry = &app.vault.entries[*i];
                let icon = match entry.kind {
                    EntryKind::Password => "🔑",
                    EntryKind::Note => "📝",
                };
                let detail = match entry.kind {
                    EntryKind::Password => {
                        if entry.username.is_empty() {
                            entry.url.clone()
                        } else if entry.url.is_empty() {
                            entry.username.clone()
                        } else {
                            format!("{} · {}", entry.username, entry.url)
                        }
                    }
                    EntryKind::Note => {
                        let preview: String = entry.notes.chars().take(60).collect();
                        preview.replace('\n', " ")
                    }
                };
                // Indent based on folder depth relative to current view
                let depth = if !app.search_query.is_empty() {
                    0
                } else if entry.folder.is_empty() {
                    0
                } else if app.current_folder.is_empty() {
                    entry.folder.matches('/').count() + 1
                } else {
                    let rel = entry.folder.strip_prefix(&format!("{}/", app.current_folder))
                        .unwrap_or("");
                    if rel.is_empty() { 1 } else { rel.matches('/').count() + 2 }
                };
                let indent = "  ".repeat(depth);
                let line = Line::from(vec![
                    Span::raw(format!(" {}{} ", indent, icon)),
                    Span::styled(
                        format!("{:<width$}", truncate_str(&entry.name, name_width), width = name_width),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        truncate_str(&detail, detail_width),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]);
                ListItem::new(line)
            }
        })
        .collect();

    let entry_count = app.filtered_indices.len();
    let total = app.vault.entries.len();
    let list_title = if !app.search_query.is_empty() {
        format!(" Results ({}/{}) ", entry_count, total)
    } else if app.current_folder.is_empty() {
        format!(" Entries ({}) ", total)
    } else {
        format!(" /{} ({}) ", app.current_folder, entry_count)
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
    if !app.list_rows.is_empty() {
        state.select(Some(app.selected));
    }
    f.render_stateful_widget(list, chunks[1], &mut state);

    // Help bar
    let help_text = match &app.status_message {
        Some((msg, _)) => Line::from(Span::styled(
            msg.as_str(),
            Style::default().fg(Color::Green),
        )),
        None => {
            let mut spans = vec![
                Span::styled(" n", Style::default().fg(Color::Yellow)),
                Span::raw(" new  "),
                Span::styled("N", Style::default().fg(Color::Yellow)),
                Span::raw(" note  "),
                Span::styled("e", Style::default().fg(Color::Yellow)),
                Span::raw(" edit  "),
                Span::styled("d", Style::default().fg(Color::Yellow)),
                Span::raw(" del  "),
                Span::styled("c", Style::default().fg(Color::Yellow)),
                Span::raw(" pw  "),
                Span::styled("u", Style::default().fg(Color::Yellow)),
                Span::raw(" user  "),
                Span::styled("/", Style::default().fg(Color::Yellow)),
                Span::raw(" search  "),
            ];
            if !app.current_folder.is_empty() {
                spans.push(Span::styled("Bksp", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" up  "));
            }
            spans.push(Span::styled("q", Style::default().fg(Color::Yellow)));
            spans.push(Span::raw(" quit"));
            Line::from(spans)
        }
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

    let mut lines = vec![Line::from("")];

    // Folder
    if !entry.folder.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Folder:     ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("/{}", &entry.folder),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled("  Name:       ", Style::default().fg(Color::Cyan)),
        Span::styled(
            &entry.name,
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ]));

    match entry.kind {
        EntryKind::Password => {
            lines.push(Line::from(vec![
                Span::styled("  Username:   ", Style::default().fg(Color::Cyan)),
                Span::raw(&entry.username),
            ]));

            // Password with reveal indicator
            lines.push(Line::from(""));
            if app.show_password {
                lines.push(Line::from(vec![
                    Span::styled("  Password:   ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        &entry.password,
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("              "),
                    Span::styled(
                        "👁  Password visible — press p to hide",
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            } else {
                let dots = "•".repeat(entry.password.len().max(12));
                lines.push(Line::from(vec![
                    Span::styled("  Password:   ", Style::default().fg(Color::Cyan)),
                    Span::styled(dots, Style::default().fg(Color::DarkGray)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("              "),
                    Span::styled(
                        "🔒 Password hidden — press p to reveal",
                        Style::default().fg(Color::Yellow),
                    ),
                ]));
            }
            lines.push(Line::from(""));

            lines.push(Line::from(vec![
                Span::styled("  URL:        ", Style::default().fg(Color::Cyan)),
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
            lines.push(Line::from(format!("    {}", note_line)));
        }
    }

    lines.push(Line::from(""));
    let modified = chrono::DateTime::from_timestamp(entry.modified_at, 0)
        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default();
    lines.push(Line::from(vec![
        Span::styled("  Modified:   ", Style::default().fg(Color::DarkGray)),
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
            Span::styled(" p", Style::default().fg(Color::Yellow)),
            Span::raw(if app.show_password { " hide pw  " } else { " reveal pw  " }),
            Span::styled("c", Style::default().fg(Color::Yellow)),
            Span::raw(" copy pw  "),
            Span::styled("u", Style::default().fg(Color::Yellow)),
            Span::raw(" copy user  "),
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

        // Folder field hint
        let hint = if *field == EntryField::Folder && is_active {
            " (e.g. Work/Email, max 3 levels)"
        } else {
            ""
        };

        if *field == EntryField::Notes {
            lines.push(Line::from(vec![
                Span::raw(indicator),
                Span::styled(format!("{:<12}", field.label()), label_style),
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
                Span::styled(format!("{:<12} ", field.label()), label_style),
                Span::raw(display_value),
                Span::styled(hint, Style::default().fg(Color::DarkGray)),
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
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}
