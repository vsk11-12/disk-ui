use humansize::{format_size, BINARY};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};

use crate::app::{App, ScanState};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Horizontal size bar — 
fn size_bar(entry_size: u64, max_size: u64, width: usize) -> String {
    let filled = if max_size > 0 {
        ((entry_size as f64 / max_size as f64) * width as f64) as usize
    } else {
        0
    }
    .min(width);
    format!("{}{}", "━".repeat(filled), "─".repeat(width - filled))
}

/// Centred popup rectangle 
fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Fill(1),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(width),
            Constraint::Fill(1),
        ])
        .split(vert[1])[1]
}

// ── Top-level render ─────────────────────────────────────────────────────────

pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(0),    // file list
            Constraint::Length(4), // selected panel
            Constraint::Length(3), // controls bar
        ])
        .split(frame.area());

    draw_header(frame, app, chunks[0]);
    draw_list(frame, app, chunks[1]);
    draw_selected(frame, app, chunks[2]);
    draw_controls(frame, app, chunks[3]);

    if app.quit_confirm {
        draw_quit_popup(frame, app);
    }
    if app.show_help {
        draw_help_popup(frame, app);
    }
}

// ── Header ────────────────────────────────────────────────────────────────────

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let t         = &app.theme;
    let scanning  = matches!(app.scan_state, ScanState::Scanning);
    let state_str = if scanning { "Scanning…" } else { "Ready" };
    let state_col = if scanning { t.scanning } else { t.yes };

    let hidden_tag = if app.show_hidden { "  [hidden: ON]" } else { "" };
    let depth      = app.nav_depth();
    let depth_str  = if depth > 0 { format!("  depth:{depth}") } else { String::new() };
    let total_sz   = format_size(app.total_size(), BINARY);

    let line = Line::from(vec![
        Span::styled(" Status: ", Style::default().fg(t.title)),
        Span::styled(
            format!("[{state_str}]"),
            Style::default().fg(state_col).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  |  ", Style::default().fg(t.dim)),
        Span::styled(
            format!("Total: {total_sz}"),
            Style::default().fg(t.size).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  |  ", Style::default().fg(t.dim)),
        Span::styled(
            format!("{}{}{}", app.current_path.display(), depth_str, hidden_tag),
            Style::default().fg(t.text),
        ),
    ]);

    frame.render_widget(
        Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(t.border))
                .title(Span::styled(" disk-ui ", Style::default().fg(t.title))),
        ),
        area,
    );
}

// ── File list ─────────────────────────────────────────────────────────────────

fn draw_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let t     = &app.theme;
    let n     = app.filtered_indices.len();
    let total = app.entries.len();

    let title = if app.is_searching {
        format!(
            " Directory (Filtering: \"{}\") [{n}/{total}]  [Esc: clear] ",
            app.search_query
        )
    } else {
        format!(" Directory ({n}/{total})  [/: search  |  .: hidden  |  r: rescan] ")
    };

    let border_color = if app.is_searching { t.selector } else { t.border };
    let title_color  = if app.is_searching { t.selector } else { t.title };

    let max_size = app.entries.first().map(|e| e.size).unwrap_or(1).max(1);
    // Column budget: 1 pad + 3 num + 1 sp + 3 tag + 1 sp + 28 name + 2 gap + bar + 2 sp + 10 size + 14 count
    let bar_w = (area.width as usize).saturating_sub(67).max(6);

    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(pos, &idx)| {
            let entry = &app.entries[idx];

            let type_tag = if entry.is_dir { "DIR" } else { "   " };

            let raw_name = if entry.is_dir {
                format!("{}/", entry.name)
            } else {
                entry.name.clone()
            };
            let max_name  = 28usize;
            let name_disp = if raw_name.chars().count() > max_name {
                format!("{}…", raw_name.chars().take(max_name - 1).collect::<String>())
            } else {
                format!("{:<width$}", raw_name, width = max_name)
            };

            let bar      = size_bar(entry.size, max_size, bar_w);
            let size_s   = format_size(entry.size, BINARY);
            let count    = if entry.is_dir {
                format!("  {:>7} items", entry.item_count)
            } else {
                String::new()
            };

            let name_style = if entry.is_dir {
                Style::default().fg(t.dir).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.text)
            };

            let tag_style = if entry.is_dir {
                Style::default().fg(t.dir).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.dim)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {:02} ", pos + 1),    Style::default().fg(t.dim)),
                Span::styled(format!("{} ", type_tag),       tag_style),
                Span::styled(name_disp,                      name_style),
                Span::raw("  "),
                Span::styled(bar,                            Style::default().fg(t.bar)),
                Span::styled(format!("  {:>10}", size_s),   Style::default().fg(t.size)),
                Span::styled(count,                          Style::default().fg(t.dim)),
            ]))
        })
        .collect();

    let display: Vec<ListItem> = if items.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            match app.scan_state {
                ScanState::Scanning => "  ⟳  Scanning directory…",
                ScanState::Done     => "  (empty directory)",
            },
            Style::default().fg(t.dim),
        )))]
    } else {
        items
    };

    let mut state = ListState::default();
    if !app.filtered_indices.is_empty() {
        state.select(Some(app.selected));
    }

    let list = List::new(display)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(Span::styled(title, Style::default().fg(title_color))),
        )
        .highlight_style(
            Style::default()
                .fg(t.highlight_fg)
                .bg(t.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut state);

    // Scrollbar — 
    if !app.filtered_indices.is_empty() {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("█")
            .style(Style::default().fg(t.border));

        let mut sb_state =
            ScrollbarState::new(app.filtered_indices.len().saturating_sub(1))
                .position(app.selected);

        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin { vertical: 1, horizontal: 0 }),
            &mut sb_state,
        );
    }
}

// ── Selected panel ───────────────────────────────────────────────────────────

fn draw_selected(frame: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;

    let content = if let Some(entry) = app.current_entry() {
        let size_s    = format_size(entry.size, BINARY);
        let kind      = if entry.is_dir { "Directory" } else { "File" };
        let count_str = if entry.is_dir {
            format!("  │  {} items inside", entry.item_count)
        } else {
            String::new()
        };

        vec![
            Line::from(vec![
                Span::styled(
                    format!(" {} ", entry.name),
                    Style::default()
                        .fg(t.highlight_fg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {kind}  │  {size_s}{count_str}"),
                    Style::default().fg(t.yes),
                ),
            ]),
            Line::from(Span::styled(
                format!(" {}", entry.path.display()),
                Style::default().fg(t.dim),
            )),
        ]
    } else {
        vec![Line::from(Span::styled(
            " No item selected",
            Style::default().fg(t.dim),
        ))]
    };

    frame.render_widget(
        Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(t.border))
                .title(Span::styled(" Selected ", Style::default().fg(t.title))),
        ),
        area,
    );
}

// ── Controls bar ─────────────────────────────────────────────────────────────

fn draw_controls(frame: &mut Frame, app: &App, area: Rect) {
    let t    = &app.theme;
    let key  = Style::default().fg(t.selector).add_modifier(Modifier::BOLD);
    let desc = Style::default().fg(t.text);
    let sep  = Span::styled("  |  ", Style::default().fg(t.dim));

    // Transient status message (scan result, toggle feedback, etc.)
    let status_span = if !app.status.is_empty() {
        Span::styled(
            format!("[{}]  ", app.status),
            Style::default().fg(t.scanning),
        )
    } else {
        Span::raw("")
    };

    let line = Line::from(vec![
        status_span,
        Span::styled("[↑/k  ↓/j]", key), Span::styled(" Navigate",      desc),
        sep.clone(),
        Span::styled("[Enter/l]",   key), Span::styled(" Enter dir",     desc),
        sep.clone(),
        Span::styled("[Bksp/h]",    key), Span::styled(" Go up",         desc),
        sep.clone(),
        Span::styled("[/]",         key), Span::styled(" Search",        desc),
        sep.clone(),
        Span::styled("[.]",         key), Span::styled(" Toggle hidden", desc),
        sep.clone(),
        Span::styled("[r]",         key), Span::styled(" Rescan",        desc),
        sep.clone(),
        Span::styled("[?]",         key), Span::styled(" Help",          desc),
        sep.clone(),
        Span::styled("[q]",         key), Span::styled(" Quit",          desc),
    ]);

    frame.render_widget(
        Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(t.border))
                .title(Span::styled(" Controls ", Style::default().fg(t.title))),
        ),
        area,
    );
}

// ── Quit popup ────────────────────────────────────────────────────────────────

fn draw_quit_popup(frame: &mut Frame, app: &App) {
    let t    = &app.theme;
    let area = centered_rect(44, 7, frame.area());
    frame.render_widget(Clear, area);

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Are you sure you want to quit?",
            Style::default().fg(t.text).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " [Y] Yes ",
                Style::default().fg(t.yes).add_modifier(Modifier::BOLD),
            ),
            Span::raw("    "),
            Span::styled(
                " [N] No  ",
                Style::default().fg(t.no).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    frame.render_widget(
        Paragraph::new(content)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(t.border))
                    .title(Span::styled(
                        " Quit Confirmation ",
                        Style::default().fg(t.title),
                    )),
            ),
        area,
    );
}

// ── Help popup ────────────────────────────────────────────────────────────────

fn draw_help_popup(frame: &mut Frame, app: &App) {
    let t    = &app.theme;
    let area = centered_rect(60, 22, frame.area());
    frame.render_widget(Clear, area);

    let key_style = Style::default()
        .fg(t.selector)
        .add_modifier(Modifier::BOLD);
    let desc_style    = Style::default().fg(t.text);
    let section_style = Style::default()
        .fg(t.yes)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);

    // Helper macro
    macro_rules! kb {
        ($k:expr, $d:expr) => {
            Line::from(vec![
                Span::styled(format!("  {:<20}", $k), key_style),
                Span::styled($d, desc_style),
            ])
        };
    }

    let content = vec![
        Line::from(Span::styled(" Navigation", section_style)),
        kb!("j / k  or  ↓ / ↑", "Move selection up / down"),
        kb!("l / Enter",          "Enter directory"),
        kb!("h / Bksp / ←",      "Go to parent directory"),
        Line::from(""),
        Line::from(Span::styled(" Search & Filter", section_style)),
        kb!("/",                  "Open search / filter mode"),
        kb!("Esc",                "Clear search and exit filter"),
        kb!(".",                  "Toggle hidden files (dotfiles)"),
        Line::from(""),
        Line::from(Span::styled(" Display", section_style)),
        kb!("r",                  "Re-scan current directory"),
        kb!("t",                  "Reload theme (theme.json)"),
        Line::from(""),
        Line::from(Span::styled(" Application", section_style)),
        kb!("?",                  "Toggle this help screen"),
        kb!("q  /  Ctrl-C",      "Quit disk-ui"),
    ];

    frame.render_widget(
        Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(t.border))
                .title(Span::styled(
                    " Keybindings  (? or Esc to close) ",
                    Style::default().fg(t.title),
                )),
        ),
        area,
    );
}
