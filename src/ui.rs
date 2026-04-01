use crate::app::{
    App, EntryField, InputMode, ListRow, PasswordChangeStep, Screen, SetupStep, SettingsItem, Tab,
    SETTINGS_ITEMS,
};
use crate::vault::{password_strength_score, EntryKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Small terminal check
    if area.width < 50 || area.height < 12 {
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                " Terminal too small ",
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!(
                " Need at least 50x12, got {}x{}",
                area.width, area.height
            )),
        ])
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(msg, area);
        return;
    }

    if matches!(app.screen, Screen::InitialUnlock | Screen::InitialSetup) {
        draw_initial_password(f, app);
        return;
    }

    if app.screen == Screen::Locked {
        draw_locked(f, app);
        return;
    }

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    draw_tab_bar(f, app, main_chunks[0]);

    match app.active_tab {
        Tab::Vault => match app.screen {
            Screen::List => draw_list(f, app, main_chunks[1]),
            Screen::ViewEntry => draw_view(f, app, main_chunks[1]),
            Screen::EditEntry => draw_edit(f, app, main_chunks[1]),
            Screen::ConfirmDelete => {
                draw_list(f, app, main_chunks[1]);
                draw_confirm_delete(f, app);
            }
            Screen::GeneratePassword => {
                draw_edit(f, app, main_chunks[1]);
                draw_generate_password(f, app);
            }
            Screen::Stats => {
                draw_list(f, app, main_chunks[1]);
                draw_stats(f, app);
            }
            _ => draw_list(f, app, main_chunks[1]),
        },
        Tab::Settings => match app.screen {
            Screen::Settings => draw_settings(f, app, main_chunks[1]),
            Screen::ChangePassword => {
                draw_settings(f, app, main_chunks[1]);
                draw_change_password(f, app);
            }
            Screen::ImportPath => {
                draw_settings(f, app, main_chunks[1]);
                draw_import_path(f, app);
            }
            _ => draw_settings(f, app, main_chunks[1]),
        },
    }
}

fn draw_tab_bar(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let titles = vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("F1", Style::default().fg(t.muted)),
            Span::raw(" Vault "),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("F2", Style::default().fg(t.muted)),
            Span::raw(" Settings "),
        ]),
    ];

    let selected_idx = match app.active_tab {
        Tab::Vault => 0,
        Tab::Settings => 1,
    };

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" GetMyPass v{} ", VERSION))
                .title_style(
                    Style::default()
                        .fg(t.accent)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .select(selected_idx)
        .style(Style::default().fg(t.muted))
        .highlight_style(
            Style::default()
                .fg(t.active)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled(" | ", Style::default().fg(t.muted)));

    f.render_widget(tabs, area);
}

// ─── List ─────────────────────────────────────────────────────────────

fn draw_list(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let width = area.width as usize;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    // Search bar
    let search_style = if app.input_mode == InputMode::Search {
        Style::default().fg(t.active)
    } else {
        Style::default().fg(t.muted)
    };
    let search_text = if app.search_query.is_empty() && app.input_mode != InputMode::Search {
        "Press / to search...".to_string()
    } else {
        app.search_query.clone()
    };

    let title = if app.current_folder.is_empty() {
        " Search ".to_string()
    } else {
        format!(" /{} - Search ", app.current_folder)
    };

    let search = Paragraph::new(search_text)
        .style(search_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(
                    Style::default()
                        .fg(t.accent)
                        .add_modifier(Modifier::BOLD),
                ),
        );
    f.render_widget(search, chunks[0]);

    if app.input_mode == InputMode::Search {
        f.set_cursor_position((
            chunks[0].x + app.search_query.len() as u16 + 1,
            chunks[0].y + 1,
        ));
    }

    let usable = width.saturating_sub(10);
    let name_width = (usable * 30 / 100).clamp(12, 45);
    let tags_width = (usable * 15 / 100).clamp(0, 20);
    let detail_width = usable.saturating_sub(name_width).saturating_sub(tags_width);

    // Entry list items
    let items: Vec<ListItem> = app
        .list_rows
        .iter()
        .map(|row| match row {
            ListRow::Folder(folder) => {
                let display_name = folder.split('/').next_back().unwrap_or(folder);
                let is_collapsed = app.collapsed_folders.iter().any(|f| f == folder);
                let arrow = if is_collapsed { ">" } else { "v" };
                let depth = folder.matches('/').count();
                let indent = "  ".repeat(depth);
                let entry_count = app
                    .vault
                    .entries
                    .iter()
                    .filter(|e| {
                        e.folder == *folder
                            || e.folder.starts_with(&format!("{}/", folder))
                    })
                    .count();
                let line = Line::from(vec![
                    Span::raw(format!(" {} ", indent)),
                    Span::styled(
                        format!("{} [D] {}", arrow, display_name),
                        Style::default()
                            .fg(t.folder)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  ({})", entry_count),
                        Style::default().fg(t.muted),
                    ),
                ]);
                ListItem::new(line)
            }
            ListRow::Entry(i) => {
                let entry = &app.vault.entries[*i];
                let fav = if entry.favorite { "*" } else { " " };
                let icon = match entry.kind {
                    EntryKind::Password => "[P]",
                    EntryKind::Note => "[N]",
                };
                let detail = match entry.kind {
                    EntryKind::Password => {
                        if entry.username.is_empty() && entry.url.is_empty() {
                            String::new()
                        } else if entry.username.is_empty() {
                            entry.url.clone()
                        } else if entry.url.is_empty() {
                            entry.username.clone()
                        } else {
                            format!("{} | {}", entry.username, entry.url)
                        }
                    }
                    EntryKind::Note => {
                        let preview: String = entry.notes.chars().take(60).collect();
                        preview.replace('\n', " ")
                    }
                };
                let tags_str = if entry.tags.is_empty() {
                    String::new()
                } else {
                    let joined: String = entry
                        .tags
                        .iter()
                        .map(|t| format!("#{}", t))
                        .collect::<Vec<_>>()
                        .join(" ");
                    truncate_str(&joined, tags_width)
                };
                let depth = if !app.search_query.is_empty() {
                    0
                } else if entry.folder.is_empty() {
                    0
                } else if app.current_folder.is_empty() {
                    entry.folder.matches('/').count() + 1
                } else {
                    let rel = entry
                        .folder
                        .strip_prefix(&format!("{}/", app.current_folder))
                        .unwrap_or("");
                    if rel.is_empty() { 1 } else { rel.matches('/').count() + 2 }
                };
                let indent = "  ".repeat(depth);
                let line = Line::from(vec![
                    Span::styled(fav, Style::default().fg(t.active)),
                    Span::raw(format!("{}{} ", indent, icon)),
                    Span::styled(
                        format!(
                            "{:<width$}",
                            truncate_str(&entry.name, name_width),
                            width = name_width
                        ),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(
                            "{:<width$}",
                            truncate_str(&detail, detail_width),
                            width = detail_width
                        ),
                        Style::default().fg(t.muted),
                    ),
                    Span::styled(tags_str, Style::default().fg(t.tag)),
                ]);
                ListItem::new(line)
            }
        })
        .collect();

    let entry_count = app.filtered_indices.len();
    let total = app.vault.entries.len();
    let sort_label = app.sort_mode.label();
    let scroll_indicator = if app.list_rows.is_empty() {
        String::new()
    } else {
        format!(" {}/{}", app.selected + 1, app.list_rows.len())
    };
    let list_title = if !app.search_query.is_empty() {
        format!(
            " Results ({}/{}) [{}]{} ",
            entry_count, total, sort_label, scroll_indicator
        )
    } else if app.current_folder.is_empty() {
        format!(" Entries ({}) [{}]{} ", total, sort_label, scroll_indicator)
    } else {
        format!(
            " /{} ({}) [{}]{} ",
            app.current_folder, entry_count, sort_label, scroll_indicator
        )
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(list_title))
        .highlight_style(
            Style::default()
                .bg(t.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    if !app.list_rows.is_empty() {
        state.select(Some(app.selected));
    }
    f.render_stateful_widget(list, chunks[1], &mut state);

    // Help bar
    let help_text = match &app.status_message {
        Some((msg, _)) => Line::from(Span::styled(
            msg.as_str(),
            Style::default().fg(t.success),
        )),
        None => Line::from(vec![
            Span::styled(" n", Style::default().fg(t.active)),
            Span::raw(" new "),
            Span::styled("e", Style::default().fg(t.active)),
            Span::raw(" edit "),
            Span::styled("d", Style::default().fg(t.active)),
            Span::raw(" del "),
            Span::styled("f", Style::default().fg(t.active)),
            Span::raw(" fav "),
            Span::styled("D", Style::default().fg(t.active)),
            Span::raw(" dup "),
            Span::styled("1", Style::default().fg(t.active)),
            Span::raw(" pw "),
            Span::styled("2", Style::default().fg(t.active)),
            Span::raw(" user "),
            Span::styled("o", Style::default().fg(t.active)),
            Span::raw(" sort "),
            Span::styled("i", Style::default().fg(t.active)),
            Span::raw(" stats "),
            Span::styled("/", Style::default().fg(t.active)),
            Span::raw(" search "),
            Span::styled("q", Style::default().fg(t.active)),
            Span::raw(" quit"),
        ]),
    };
    let help = Paragraph::new(help_text).block(
        Block::default().borders(Borders::ALL).title(" Help "),
    );
    f.render_widget(help, chunks[2]);
}

// ─── View Entry ───────────────────────────────────────────────────────

fn draw_view(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let entry = match app.selected_entry() {
        Some(e) => e,
        None => return,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let inner_width = chunks[0].width.saturating_sub(2) as usize;
    let separator = "─".repeat(inner_width.saturating_sub(4));

    let icon = match entry.kind {
        EntryKind::Password => "[P]",
        EntryKind::Note => "[N]",
    };
    let fav = if entry.favorite { " ★" } else { "" };

    let mut lines: Vec<Line> = Vec::new();

    // ── Header section ──
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{} {}{}", icon, entry.name, fav),
            Style::default()
                .fg(t.accent)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    if !entry.folder.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("/{}", &entry.folder),
                Style::default().fg(t.folder),
            ),
        ]));
    }

    if !entry.tags.is_empty() {
        let tags_display = entry
            .tags
            .iter()
            .map(|tg| format!("#{}", tg))
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(tags_display, Style::default().fg(t.tag)),
        ]));
    }

    lines.push(Line::from(Span::styled(
        format!("  {}", separator),
        Style::default().fg(t.muted),
    )));

    // ── Credentials section ──
    match entry.kind {
        EntryKind::Password => {
            if !entry.username.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("  User  ", Style::default().fg(t.muted)),
                    Span::raw(&entry.username),
                ]));
            }

            if app.show_password {
                lines.push(Line::from(vec![
                    Span::styled("  Pass  ", Style::default().fg(t.muted)),
                    Span::styled(
                        &entry.password,
                        Style::default()
                            .fg(t.success)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                let (_, strength_label, strength_bar) =
                    password_strength_score(&entry.password);
                let strength_color = strength_color_from_label(strength_label);
                lines.push(Line::from(vec![
                    Span::raw("        "),
                    Span::styled(
                        format!("{} {}", strength_bar, strength_label),
                        Style::default().fg(strength_color),
                    ),
                ]));
            } else {
                let dots = "●".repeat(entry.password.len().clamp(8, 16));
                lines.push(Line::from(vec![
                    Span::styled("  Pass  ", Style::default().fg(t.muted)),
                    Span::styled(dots, Style::default().fg(t.muted)),
                    Span::styled(
                        "  p to reveal",
                        Style::default().fg(t.active),
                    ),
                ]));
            }

            if !entry.url.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("  URL   ", Style::default().fg(t.muted)),
                    Span::raw(&entry.url),
                ]));
            }

            // TOTP
            if !entry.totp_secret.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("  {}", separator),
                    Style::default().fg(t.muted),
                )));
                if let Some((code, remaining)) =
                    crate::totp::generate_totp(&entry.totp_secret)
                {
                    let formatted_code =
                        format!("{} {}", &code[..3], &code[3..]);
                    let filled = (remaining as usize * 20 / 30).min(20);
                    let empty = 20 - filled;
                    let bar = format!(
                        "{}{}",
                        "█".repeat(filled),
                        "░".repeat(empty)
                    );
                    let bar_color = if remaining <= 5 {
                        t.error
                    } else if remaining <= 10 {
                        t.active
                    } else {
                        t.success
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  TOTP  ", Style::default().fg(t.muted)),
                        Span::styled(
                            formatted_code,
                            Style::default()
                                .fg(t.success)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("  {}s ", remaining),
                            Style::default().fg(t.muted),
                        ),
                        Span::styled(bar, Style::default().fg(bar_color)),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("  TOTP  ", Style::default().fg(t.muted)),
                        Span::styled("Invalid secret", Style::default().fg(t.error)),
                    ]));
                }
            }

            // Password history
            if !entry.password_history.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("  {}", separator),
                    Style::default().fg(t.muted),
                )));
                if app.show_history {
                    lines.push(Line::from(vec![
                        Span::styled("  History ", Style::default().fg(t.muted)),
                        Span::styled(
                            format!("({} entries)", entry.password_history.len()),
                            Style::default().fg(t.muted),
                        ),
                    ]));
                    for item in entry.password_history.iter().rev().take(5) {
                        let date = chrono::DateTime::from_timestamp(item.changed_at, 0)
                            .map(|d| d.format("%Y-%m-%d").to_string())
                            .unwrap_or_default();
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("    {} ", date),
                                Style::default().fg(t.muted),
                            ),
                            Span::styled(
                                &item.password,
                                Style::default().fg(t.error),
                            ),
                        ]));
                    }
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("  History ", Style::default().fg(t.muted)),
                        Span::styled(
                            format!("{} entries ", entry.password_history.len()),
                            Style::default().fg(t.muted),
                        ),
                        Span::styled("H to show", Style::default().fg(t.active)),
                    ]));
                }
            }
        }
        EntryKind::Note => {}
    }

    // ── Notes section ──
    if !entry.notes.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  {}", separator),
            Style::default().fg(t.muted),
        )));
        lines.push(Line::from(Span::styled(
            "  Notes",
            Style::default().fg(t.muted),
        )));
        for note_line in entry.notes.lines() {
            lines.push(Line::from(format!("  {}", note_line)));
        }
    }

    // ── Footer ──
    lines.push(Line::from(""));
    let modified = chrono::DateTime::from_timestamp(entry.modified_at, 0)
        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default();
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("Modified {}", modified),
            Style::default().fg(t.muted),
        ),
    ]));

    let title = format!(" {} Detail ", icon);
    let view = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(
                    Style::default()
                        .fg(t.accent)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(view, chunks[0]);

    // Help bar
    let help_text = match &app.status_message {
        Some((msg, _)) => {
            Line::from(Span::styled(msg.as_str(), Style::default().fg(t.success)))
        }
        None => Line::from(vec![
            Span::styled(" p", Style::default().fg(t.active)),
            Span::raw(if app.show_password {
                " hide"
            } else {
                " reveal"
            }),
            Span::styled("  1", Style::default().fg(t.active)),
            Span::raw(" pw"),
            Span::styled("  2", Style::default().fg(t.active)),
            Span::raw(" user"),
            Span::styled("  f", Style::default().fg(t.active)),
            Span::raw(" fav"),
            Span::styled("  e", Style::default().fg(t.active)),
            Span::raw(" edit"),
            Span::styled("  H", Style::default().fg(t.active)),
            Span::raw(" history"),
            Span::styled("  Esc", Style::default().fg(t.active)),
            Span::raw(" back"),
        ]),
    };
    let help = Paragraph::new(help_text).block(
        Block::default().borders(Borders::ALL),
    );
    f.render_widget(help, chunks[1]);
}

// ─── Edit Entry ───────────────────────────────────────────────────────

fn draw_edit(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let fields = app.current_fields();
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for (i, field) in fields.iter().enumerate() {
        let is_active = i == app.active_field;
        let label_style = if is_active {
            Style::default()
                .fg(t.active)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.accent)
        };

        let value = app.get_field_value(field);
        let display_value = if *field == EntryField::Password && !app.show_password {
            "*".repeat(value.len())
        } else {
            value.to_string()
        };

        let indicator = if is_active { "> " } else { "  " };

        let hint = match field {
            EntryField::Folder if is_active => " (e.g. Work/Email, max 3 levels)",
            EntryField::TotpSecret if is_active => " (base32 secret from authenticator)",
            EntryField::Tags if is_active => " (comma separated: work, email, dev)",
            _ => "",
        };

        if *field == EntryField::Notes {
            lines.push(Line::from(vec![
                Span::raw(indicator),
                Span::styled(format!("{:<14}", field.label()), label_style),
            ]));
            for note_line in display_value.lines() {
                lines.push(Line::from(format!("    {}", note_line)));
            }
            if display_value.is_empty() || display_value.ends_with('\n') {
                lines.push(Line::from("    "));
            }
        } else {
            let mut spans = vec![
                Span::raw(indicator),
                Span::styled(format!("{:<14} ", field.label()), label_style),
                Span::raw(display_value.clone()),
                Span::styled(hint.to_string(), Style::default().fg(t.muted)),
            ];

            // Password strength indicator inline
            if *field == EntryField::Password && !value.is_empty() {
                let (_, label, bar) = password_strength_score(value);
                let color = strength_color_from_label(label);
                spans.push(Span::styled(
                    format!("  {} {}", bar, label),
                    Style::default().fg(color),
                ));
            }

            lines.push(Line::from(spans));
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
                .title_style(
                    Style::default()
                        .fg(t.active)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(edit, chunks[0]);

    // Help — F5/F6 instead of Ctrl+S/G for SSH compatibility
    let help_text = Line::from(vec![
        Span::styled(" Tab", Style::default().fg(t.active)),
        Span::raw(" next "),
        Span::styled("S-Tab", Style::default().fg(t.active)),
        Span::raw(" prev "),
        Span::styled("F5", Style::default().fg(t.active)),
        Span::raw(" save "),
        Span::styled("F6", Style::default().fg(t.active)),
        Span::raw(" gen pw "),
        Span::styled("Esc", Style::default().fg(t.active)),
        Span::raw(" cancel"),
    ]);
    let help = Paragraph::new(help_text).block(
        Block::default().borders(Borders::ALL).title(" Help "),
    );
    f.render_widget(help, chunks[1]);
}

// ─── Confirm Delete ───────────────────────────────────────────────────

fn draw_confirm_delete(f: &mut Frame, app: &App) {
    let t = &app.theme;
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
            Span::styled(
                "  y",
                Style::default()
                    .fg(t.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" yes  "),
            Span::styled(
                "n",
                Style::default()
                    .fg(t.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" no"),
        ]),
    ];

    let popup = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Confirm Delete ")
            .title_style(
                Style::default()
                    .fg(t.error)
                    .add_modifier(Modifier::BOLD),
            ),
    );
    f.render_widget(popup, area);
}

// ─── Generate Password ───────────────────────────────────────────────

fn draw_generate_password(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = centered_rect(60, 50, f.area());
    f.render_widget(Clear, area);

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Length: ", Style::default().fg(t.accent)),
            Span::styled(
                format!("{}", app.gen_length),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("  (</>)", Style::default().fg(t.muted)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            checkbox_themed(app.gen_uppercase, t),
            Span::styled(" 1", Style::default().fg(t.active)),
            Span::raw(" Uppercase (A-Z)"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            checkbox_themed(app.gen_lowercase, t),
            Span::styled(" 2", Style::default().fg(t.active)),
            Span::raw(" Lowercase (a-z)"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            checkbox_themed(app.gen_digits, t),
            Span::styled(" 3", Style::default().fg(t.active)),
            Span::raw(" Digits (0-9)"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            checkbox_themed(app.gen_symbols, t),
            Span::styled(" 4", Style::default().fg(t.active)),
            Span::raw(" Symbols (!@#$...)"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Preview: ", Style::default().fg(t.accent)),
            Span::styled(
                &app.gen_preview,
                Style::default()
                    .fg(t.success)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(t.active)),
            Span::raw(" accept  "),
            Span::styled("r", Style::default().fg(t.active)),
            Span::raw(" regenerate  "),
            Span::styled("Esc", Style::default().fg(t.active)),
            Span::raw(" cancel"),
        ]),
    ];

    let popup = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Generate Password ")
            .title_style(
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD),
            ),
    );
    f.render_widget(popup, area);
}

// ─── Stats ────────────────────────────────────────────────────────────

fn draw_stats(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = centered_rect(55, 65, f.area());
    f.render_widget(Clear, area);

    let stats = app.compute_stats();

    let weak_color = if stats.weak_passwords > 0 {
        t.error
    } else {
        t.success
    };
    let dup_color = if stats.duplicate_passwords > 0 {
        t.error
    } else {
        t.success
    };

    let text = vec![
        Line::from(""),
        stat_line("Total entries", stats.total.to_string(), Color::White),
        stat_line("Passwords", stats.passwords.to_string(), t.accent),
        stat_line("Notes", stats.notes.to_string(), t.accent),
        Line::from(""),
        stat_line("Favorites", stats.favorites.to_string(), t.active),
        stat_line("Folders", stats.folders.to_string(), t.folder),
        stat_line("Tags", stats.tags_count.to_string(), t.tag),
        stat_line("TOTP entries", stats.totp_count.to_string(), t.accent),
        Line::from(""),
        stat_line("Weak passwords", stats.weak_passwords.to_string(), weak_color),
        stat_line("Reused passwords", stats.duplicate_passwords.to_string(), dup_color),
        Line::from(""),
        stat_line("Avg age (days)", stats.avg_age_days.to_string(), t.muted),
        stat_line("Oldest (days)", stats.oldest_days.to_string(), t.muted),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Esc", Style::default().fg(t.active)),
            Span::raw(" close"),
        ]),
    ];

    let popup = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Vault Statistics ")
            .title_style(
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD),
            ),
    );
    f.render_widget(popup, area);
}

fn stat_line(label: &str, value: impl Into<String>, color: Color) -> Line<'static> {
    let l = format!("  {:<22}", label);
    let v: String = value.into();
    Line::from(vec![
        Span::raw(l),
        Span::styled(v, Style::default().fg(color).add_modifier(Modifier::BOLD)),
    ])
}

// ─── Settings ─────────────────────────────────────────────────────────

fn draw_settings(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    let mut current_section = "";
    let mut selected_line: usize = 0;

    for (i, item) in SETTINGS_ITEMS.iter().enumerate() {
        let section = item.section();
        if section != current_section {
            if !current_section.is_empty() {
                lines.push(Line::from(""));
            }
            lines.push(Line::from(Span::styled(
                format!("  -- {} --", section),
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            current_section = section;
        }

        let is_selected = i == app.settings_selected;
        if is_selected {
            selected_line = lines.len();
        }
        let indicator = if is_selected { "> " } else { "  " };
        let label_style = if is_selected {
            Style::default()
                .fg(t.active)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let value = settings_value_display(app, item);
        let hint = settings_hint(item);

        lines.push(Line::from(vec![
            Span::raw(format!("  {}", indicator)),
            Span::styled(format!("{:<25}", item.label()), label_style),
            Span::styled(value, Style::default().fg(t.success)),
            Span::styled(hint, Style::default().fg(t.muted)),
        ]));
    }

    // Info section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  -- Info --",
        Style::default()
            .fg(t.accent)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled(
            "Version               ",
            Style::default().fg(t.muted),
        ),
        Span::raw(VERSION),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled(
            "Vault location        ",
            Style::default().fg(t.muted),
        ),
        Span::raw("~/.gmp/vault.enc"),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled(
            "Entries               ",
            Style::default().fg(t.muted),
        ),
        Span::raw(format!("{}", app.vault.entries.len())),
    ]));
    let backup_exists = crate::vault::vault_dir().join("vault.enc.bak").exists();
    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled(
            "Backup available      ",
            Style::default().fg(t.muted),
        ),
        Span::raw(if backup_exists { "Yes" } else { "No" }),
    ]));

    // Calculate scroll offset to keep selected item visible
    // Inner height = chunk height - 2 (borders)
    let inner_height = chunks[0].height.saturating_sub(2) as usize;
    let scroll_offset = if inner_height > 0 && selected_line >= inner_height {
        (selected_line - inner_height + 3) as u16 // +3 for some padding
    } else {
        0
    };

    let settings_widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Settings ")
                .title_style(
                    Style::default()
                        .fg(t.active)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .scroll((scroll_offset, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(settings_widget, chunks[0]);

    let help_text = match &app.status_message {
        Some((msg, _)) => {
            Line::from(Span::styled(msg.as_str(), Style::default().fg(t.success)))
        }
        None => Line::from(vec![
            Span::styled(" j/k", Style::default().fg(t.active)),
            Span::raw(" navigate  "),
            Span::styled("</>", Style::default().fg(t.active)),
            Span::raw(" adjust  "),
            Span::styled("Enter", Style::default().fg(t.active)),
            Span::raw(" action  "),
            Span::styled("Tab", Style::default().fg(t.active)),
            Span::raw(" vault  "),
            Span::styled("q", Style::default().fg(t.active)),
            Span::raw(" quit"),
        ]),
    };
    let help = Paragraph::new(help_text).block(
        Block::default().borders(Borders::ALL).title(" Help "),
    );
    f.render_widget(help, chunks[1]);
}

fn settings_value_display(app: &App, item: &SettingsItem) -> String {
    match item {
        SettingsItem::AutoLock => app.config.auto_lock_label().to_string(),
        SettingsItem::ClipboardClear => app.config.clipboard_clear_label().to_string(),
        SettingsItem::AutoBackup => {
            if app.config.backup_enabled {
                "[x] On".to_string()
            } else {
                "[ ] Off".to_string()
            }
        }
        SettingsItem::ThemeSetting => app.config.theme.clone(),
        SettingsItem::DefaultGenLength => format!("{} chars", app.config.default_gen_length),
        SettingsItem::ChangePassword => "***".to_string(),
        SettingsItem::ExportJson => "~/.gmp/export.json".to_string(),
        SettingsItem::ImportJson => "Select file...".to_string(),
    }
}

fn settings_hint(item: &SettingsItem) -> String {
    match item {
        SettingsItem::AutoLock
        | SettingsItem::ClipboardClear
        | SettingsItem::ThemeSetting
        | SettingsItem::DefaultGenLength => "  (</>)".to_string(),
        _ => "  (Enter)".to_string(),
    }
}

// ─── Locked ───────────────────────────────────────────────────────────

fn draw_initial_password(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = f.area();
    let popup_area = centered_rect(60, 40, area);

    let bg = Paragraph::new("").style(Style::default().bg(Color::Black));
    f.render_widget(bg, area);
    f.render_widget(Clear, popup_area);

    let is_unlock = app.screen == Screen::InitialUnlock;

    let mut lines = vec![
        Line::from(""),
        Line::from(""),
    ];

    if is_unlock {
        lines.push(Line::from(Span::styled(
            "       ENTER MASTER PASSWORD",
            Style::default()
                .fg(t.active)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("   Password: ", Style::default().fg(t.accent)),
            Span::raw("*".repeat(app.initial_password_input.len())),
        ]));
        lines.push(Line::from(""));
    } else {
        let step_label = match app.initial_setup_step {
            SetupStep::NewPassword => "Choose a master password (min 8 chars)",
            SetupStep::ConfirmPassword => "Confirm master password",
        };
        lines.push(Line::from(Span::styled(
            "       CREATE NEW VAULT",
            Style::default()
                .fg(t.active)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("   {}", step_label),
            Style::default().fg(t.accent),
        )));
        lines.push(Line::from(""));

        let input = match app.initial_setup_step {
            SetupStep::NewPassword => &app.initial_password_input,
            SetupStep::ConfirmPassword => &app.initial_password_confirm,
        };
        lines.push(Line::from(vec![
            Span::styled("   Password: ", Style::default().fg(t.accent)),
            Span::raw("*".repeat(input.len())),
        ]));
        lines.push(Line::from(""));

        if app.initial_setup_step == SetupStep::NewPassword
            && !app.initial_password_input.is_empty()
        {
            let (_, label, bar) = password_strength_score(&app.initial_password_input);
            let color = strength_color_from_label(label);
            lines.push(Line::from(vec![
                Span::styled("   Strength: ", Style::default().fg(t.muted)),
                Span::styled(
                    format!("{} {}", bar, label),
                    Style::default().fg(color),
                ),
            ]));
            lines.push(Line::from(""));
        }
    }

    if let Some(err) = &app.initial_error {
        lines.push(Line::from(Span::styled(
            format!("   {}", err),
            Style::default().fg(t.error),
        )));
        lines.push(Line::from(""));
    }

    if is_unlock {
        lines.push(Line::from(vec![
            Span::styled("   Enter", Style::default().fg(t.active)),
            Span::raw(" unlock  "),
            Span::styled("Esc", Style::default().fg(t.active)),
            Span::raw(" quit"),
        ]));
    } else {
        match app.initial_setup_step {
            SetupStep::NewPassword => {
                lines.push(Line::from(vec![
                    Span::styled("   Enter", Style::default().fg(t.active)),
                    Span::raw(" continue  "),
                    Span::styled("Esc", Style::default().fg(t.active)),
                    Span::raw(" quit"),
                ]));
            }
            SetupStep::ConfirmPassword => {
                lines.push(Line::from(vec![
                    Span::styled("   Enter", Style::default().fg(t.active)),
                    Span::raw(" confirm  "),
                    Span::styled("Esc", Style::default().fg(t.active)),
                    Span::raw(" back"),
                ]));
            }
        }
    }

    let title = if is_unlock {
        " GetMyPass "
    } else {
        " GetMyPass - New Vault "
    };

    let popup = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD),
            ),
    );
    f.render_widget(popup, popup_area);
}

fn draw_locked(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = f.area();
    let popup_area = centered_rect(60, 40, area);

    let bg = Paragraph::new("").style(Style::default().bg(Color::Black));
    f.render_widget(bg, area);
    f.render_widget(Clear, popup_area);

    let mut lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "       VAULT LOCKED",
            Style::default()
                .fg(t.active)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "   Session timed out due to inactivity.",
            Style::default().fg(t.muted),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("   Password: ", Style::default().fg(t.accent)),
            Span::raw("*".repeat(app.lock_password_input.len())),
        ]),
        Line::from(""),
    ];

    if let Some(err) = &app.lock_error {
        lines.push(Line::from(Span::styled(
            format!("   {}", err),
            Style::default().fg(t.error),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![
        Span::styled("   Enter", Style::default().fg(t.active)),
        Span::raw(" unlock  "),
        Span::styled("Esc", Style::default().fg(t.active)),
        Span::raw(" quit"),
    ]));

    let popup = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" GetMyPass - Locked ")
            .title_style(
                Style::default()
                    .fg(t.error)
                    .add_modifier(Modifier::BOLD),
            ),
    );
    f.render_widget(popup, popup_area);
}

// ─── Change Password ──────────────────────────────────────────────────

fn draw_change_password(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = centered_rect(60, 45, f.area());
    f.render_widget(Clear, area);

    let step_label = match app.pw_change_step {
        PasswordChangeStep::CurrentPassword => "Step 1/3: Enter current password",
        PasswordChangeStep::NewPassword => "Step 2/3: Enter new password (min 8 chars)",
        PasswordChangeStep::ConfirmPassword => "Step 3/3: Confirm new password",
    };

    let input = match app.pw_change_step {
        PasswordChangeStep::CurrentPassword => &app.pw_change_current,
        PasswordChangeStep::NewPassword => &app.pw_change_new,
        PasswordChangeStep::ConfirmPassword => &app.pw_change_confirm,
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", step_label),
            Style::default()
                .fg(t.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  > "),
            Span::raw("*".repeat(input.len())),
        ]),
        Line::from(""),
    ];

    if app.pw_change_step == PasswordChangeStep::NewPassword && !app.pw_change_new.is_empty() {
        let (_, label, bar) = password_strength_score(&app.pw_change_new);
        let color = strength_color_from_label(label);
        lines.push(Line::from(vec![
            Span::styled("  Strength: ", Style::default().fg(t.muted)),
            Span::styled(
                format!("{} {}", bar, label),
                Style::default().fg(color),
            ),
        ]));
        lines.push(Line::from(""));
    }

    if let Some(err) = &app.pw_change_error {
        lines.push(Line::from(Span::styled(
            format!("  {}", err),
            Style::default().fg(t.error),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![
        Span::styled("  Enter", Style::default().fg(t.active)),
        Span::raw(" continue  "),
        Span::styled("Esc", Style::default().fg(t.active)),
        Span::raw(" cancel"),
    ]));

    let popup = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Change Master Password ")
            .title_style(
                Style::default()
                    .fg(t.active)
                    .add_modifier(Modifier::BOLD),
            ),
    );
    f.render_widget(popup, area);
}

// ─── Import ───────────────────────────────────────────────────────────

fn draw_import_path(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = centered_rect(65, 35, f.area());
    f.render_widget(Clear, area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Enter the path to a JSON file to import:",
            Style::default().fg(t.accent),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  > "),
            Span::styled(
                &app.import_path_input,
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Format: JSON array of entries (from export)",
            Style::default().fg(t.muted),
        )),
        Line::from(Span::styled(
            "  Tip: use ~/path for home-relative paths",
            Style::default().fg(t.muted),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(t.active)),
            Span::raw(" import  "),
            Span::styled("Esc", Style::default().fg(t.active)),
            Span::raw(" cancel"),
        ]),
    ];

    let popup = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Import Entries ")
            .title_style(
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD),
            ),
    );
    f.render_widget(popup, area);
}

// ─── Helpers ──────────────────────────────────────────────────────────

fn strength_color_from_label(label: &str) -> Color {
    match label {
        "Strong" => Color::Green,
        "Good" => Color::LightGreen,
        "Fair" => Color::Yellow,
        "Weak" | "Too short" => Color::Red,
        _ => Color::DarkGray,
    }
}

fn checkbox_themed(checked: bool, t: &crate::theme::Theme) -> Span<'static> {
    if checked {
        Span::styled("[x]", Style::default().fg(t.success))
    } else {
        Span::styled("[ ]", Style::default().fg(t.muted))
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
    if max == 0 {
        return String::new();
    }
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}~", truncated)
    }
}
