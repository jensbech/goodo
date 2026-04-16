use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::app::{App, DisplayItem, Mode};

const ZEBRA_DARK: Color = Color::Rgb(30, 30, 40);
const HIGHLIGHT_BG: Color = Color::Rgb(55, 55, 80);
const DIM: Color = Color::Rgb(100, 100, 110);
const ACCENT: Color = Color::Rgb(180, 180, 255);
const DETAIL_BG: Color = Color::Rgb(10, 8, 22);
const DONE_BG: Color = Color::Rgb(18, 34, 18);
const MAX_WIDTH: u16 = 140;

pub fn draw(f: &mut Frame, app: &App) {
    let full = f.area();

    let content_area = if full.width > MAX_WIDTH {
        let x = full.x + (full.width - MAX_WIDTH) / 2;
        Rect::new(x, full.y, MAX_WIDTH, full.height)
    } else {
        full
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(match app.mode {
                Mode::Adding
                | Mode::AddingSubtask
                | Mode::AddingSection
                | Mode::Editing
                | Mode::EditingSection => 3,
                _ => 1,
            }),
        ])
        .split(content_area);

    draw_list(f, app, chunks[0]);

    match app.mode {
        Mode::Adding => draw_input(f, app, chunks[1], "Add todo"),
        Mode::AddingSubtask => draw_input(f, app, chunks[1], "Add subtask"),
        Mode::AddingSection => draw_input(f, app, chunks[1], "New section"),
        Mode::Editing => draw_input(f, app, chunks[1], "Edit todo"),
        Mode::EditingSection => draw_input(f, app, chunks[1], "Rename section"),
        _ => draw_footer(f, app, chunks[1]),
    }

    match app.mode {
        Mode::ConfirmDelete => draw_confirm_delete(f, app, content_area),
        Mode::ConfirmDeleteSection => draw_confirm_delete_section(f, app, content_area),
        _ => {}
    }
}

fn draw_list(f: &mut Frame, app: &App, area: Rect) {
    let flat = app.flat_view();
    let todo_count = flat.iter().filter(|i| matches!(i, DisplayItem::Todo(_))).count();
    let done_count = flat.iter().filter(|i| {
        if let DisplayItem::Todo(ti) = i { app.todos[*ti].done } else { false }
    }).count();

    let version = env!("CARGO_PKG_VERSION");
    let title = if todo_count == 0 {
        format!(" goodo v{version} ")
    } else {
        format!(" goodo v{version}  {done_count}/{todo_count} ✓ ")
    };

    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)

        .border_style(Style::default().fg(Color::Rgb(60, 60, 90)))
        .style(Style::default().bg(Color::Rgb(18, 18, 28)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if flat.is_empty() {
        let hint = Paragraph::new(Span::styled(
            "No todos yet. Press [a] to add one.",
            Style::default().fg(DIM),
        ))
        .alignment(Alignment::Center);
        let y = inner.y + inner.height / 2;
        f.render_widget(hint, Rect { x: inner.x, y, width: inner.width, height: 1 });
        return;
    }

    let available_width = inner.width as usize;

    let items: Vec<ListItem> = flat
        .iter()
        .enumerate()
        .map(|(display_i, item)| {
            let is_selected = display_i == app.selected;

            match item {
                DisplayItem::SectionHeading(si) => {
                    let name = &app.sections[*si].name;
                    let bg = if is_selected { HIGHLIGHT_BG } else { Color::Reset };

                    let prefix = if is_selected { "▶ " } else { "  " };
                    let dashes = "── ";
                    let suffix_start = prefix.len() + dashes.len() + name.len() + 1;
                    let fill_count = available_width.saturating_sub(suffix_start);
                    let fill = "─".repeat(fill_count);

                    let (prefix_style, dash_style, name_style, fill_style) = if is_selected {
                        (
                            Style::default().fg(ACCENT).bg(bg),
                            Style::default().fg(ACCENT).bg(bg),
                            Style::default().fg(ACCENT).bg(bg).add_modifier(Modifier::BOLD),
                            Style::default().fg(Color::Rgb(80, 80, 120)).bg(bg),
                        )
                    } else {
                        (
                            Style::default().fg(Color::Rgb(70, 70, 100)).bg(bg),
                            Style::default().fg(Color::Rgb(60, 60, 90)).bg(bg),
                            Style::default().fg(Color::Rgb(150, 150, 185)).bg(bg).add_modifier(Modifier::BOLD),
                            Style::default().fg(Color::Rgb(45, 45, 70)).bg(bg),
                        )
                    };

                    ListItem::new(Line::from(vec![
                        Span::styled(prefix, prefix_style),
                        Span::styled(dashes, dash_style),
                        Span::styled(name.clone(), name_style),
                        Span::styled(format!(" {fill}"), fill_style),
                    ]))
                }

                DisplayItem::Todo(ti) => {
                    let todo = &app.todos[*ti];
                    let is_subtask = todo.parent_id.is_some();
                    let is_odd = display_i % 2 == 1;

                    let bg = if is_selected {
                        HIGHLIGHT_BG
                    } else if todo.done {
                        DONE_BG
                    } else if is_odd {
                        ZEBRA_DARK
                    } else {
                        Color::Reset
                    };

                    let prefix = if is_selected { "▶ " } else if todo.done { "✓ " } else { "· " };

                    let prefix_color = if is_selected {
                        ACCENT
                    } else if todo.done {
                        Color::Rgb(80, 185, 80)
                    } else if is_subtask {
                        Color::Rgb(120, 120, 145)
                    } else {
                        Color::Rgb(140, 140, 160)
                    };

                    let text_style = if is_selected {
                        Style::default().fg(ACCENT).bg(bg)
                    } else if todo.done {
                        Style::default().fg(Color::Rgb(75, 155, 75)).add_modifier(Modifier::CROSSED_OUT).bg(bg)
                    } else if is_subtask {
                        Style::default().fg(Color::Rgb(185, 185, 205)).bg(bg)
                    } else {
                        Style::default().fg(Color::Rgb(210, 210, 225)).bg(bg)
                    };

                    let mut spans = Vec::new();
                    if is_subtask {
                        spans.push(Span::styled("  └ ", Style::default().fg(Color::Rgb(70, 70, 95)).bg(bg)));
                    }
                    spans.push(Span::styled(prefix, Style::default().fg(prefix_color).bg(bg)));
                    spans.push(Span::styled(todo.text.clone(), text_style));

                    if !is_subtask {
                        let total_sub = app.todos.iter().filter(|t| t.parent_id == Some(todo.id)).count();
                        if total_sub > 0 {
                            let done_sub = app.todos.iter()
                                .filter(|t| t.parent_id == Some(todo.id) && t.done)
                                .count();
                            let badge_color = if done_sub == total_sub {
                                Color::Rgb(80, 185, 80)
                            } else {
                                Color::Rgb(120, 120, 145)
                            };
                            spans.push(Span::styled(
                                format!("  ({done_sub}/{total_sub})"),
                                Style::default().fg(badge_color).bg(bg),
                            ));
                        }
                    }

                    ListItem::new(Line::from(spans))
                }
            }
        })
        .collect();

    f.render_widget(List::new(items), inner);
}

fn draw_input(f: &mut Frame, app: &App, area: Rect, title: &str) {
    let visible = visible_input(&app.input, app.cursor_pos, area.width.saturating_sub(4) as usize);
    let block = Block::default()
        .title(Span::styled(format!(" {title} "), Style::default().fg(ACCENT)))
        .borders(Borders::ALL)

        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(DETAIL_BG));
    f.render_widget(Paragraph::new(visible).block(block), area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let on_section = app.selected_section_idx().is_some();
    let selected_is_top = app.selected_todo_idx()
        .map(|i| app.todos[i].parent_id.is_none())
        .unwrap_or(false);

    let s = Style::default();
    let key = |k: &'static str| Span::styled(k, s.fg(ACCENT).add_modifier(Modifier::BOLD));
    let lbl = |l: &'static str| Span::styled(l, s.fg(DIM));

    let hints: Vec<Span> = if on_section {
        vec![
            key("[a]"), lbl("dd  "),
            key("[n]"), lbl("ew section  "),
            key("[e]"), lbl("dit  "),
            key("[d]"), lbl("el  "),
            key("[J/K]"), lbl(" move  "),
            key("[u]"), lbl("ndo  "),
            key("[r]"), lbl("edo  "),
            key("[q]"), lbl("uit"),
        ]
    } else if selected_is_top {
        vec![
            key("[a]"), lbl("dd  "),
            key("[A]"), lbl(" subtask  "),
            key("[n]"), lbl(" section  "),
            key("[e]"), lbl("dit  "),
            key("[J/K]"), lbl(" move  "),
            key("[tab]"), lbl(" indent  "),
            key("[spc]"), lbl(" toggle  "),
            key("[x]"), lbl("/"), key("[d]"), lbl("el  "),
            key("[u]"), lbl("ndo  "),
            key("[r]"), lbl("edo  "),
            key("[q]"), lbl("uit"),
        ]
    } else {
        vec![
            key("[a]"), lbl("dd  "),
            key("[n]"), lbl(" section  "),
            key("[e]"), lbl("dit  "),
            key("[J/K]"), lbl(" move  "),
            key("[tab]"), lbl(" dedent  "),
            key("[spc]"), lbl(" toggle  "),
            key("[x]"), lbl("/"), key("[d]"), lbl("el  "),
            key("[u]"), lbl("ndo  "),
            key("[r]"), lbl("edo  "),
            key("[q]"), lbl("uit"),
        ]
    };

    let para = Paragraph::new(Line::from(hints))
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Rgb(18, 18, 28)));
    f.render_widget(para, area);
}

fn draw_confirm_delete(f: &mut Frame, app: &App, area: Rect) {
    let Some(idx) = app.selected_todo_idx() else { return };
    let todo = &app.todos[idx];
    let has_children = app.todos.iter().any(|t| t.parent_id == Some(todo.id));
    let extra = if has_children { " + subtasks" } else { "" };
    let text = format!(" Delete \"{}\"{extra}? [y]es / [n]o ", todo.text);
    draw_modal(f, area, &text, Color::Rgb(200, 80, 80));
}

fn draw_confirm_delete_section(f: &mut Frame, app: &App, area: Rect) {
    let Some(si) = app.selected_section_idx() else { return };
    let section = &app.sections[si];
    let count = app.todos.iter().filter(|t| t.section_id == section.id).count();
    let extra = if count > 0 { format!(" + {count} todos") } else { String::new() };
    let text = format!(" Delete section \"{}\"{extra}? [y]es / [n]o ", section.name);
    draw_modal(f, area, &text, Color::Rgb(200, 80, 80));
}

fn draw_modal(f: &mut Frame, area: Rect, text: &str, border_color: Color) {
    let width = (text.chars().count() as u16 + 4).min(area.width.saturating_sub(4));
    let height = 3u16;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect { x, y, width, height };

    let buf = f.buffer_mut();
    for row in area.y..area.y + area.height {
        for col in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((col, row)) {
                cell.set_fg(Color::Rgb(50, 50, 60));
                cell.set_bg(Color::Rgb(10, 10, 15));
            }
        }
    }

    f.render_widget(Clear, modal_area);
    let block = Block::default()
        .borders(Borders::ALL)

        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(DETAIL_BG));
    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);
    f.render_widget(
        Paragraph::new(Span::styled(text.to_owned(), Style::default().fg(Color::Rgb(220, 200, 200))))
            .alignment(Alignment::Center),
        inner,
    );
}

fn visible_input(input: &str, cursor_pos: usize, max_width: usize) -> Line<'static> {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();

    let (start, display_cursor) = if cursor_pos <= max_width.saturating_sub(1) {
        (0, cursor_pos)
    } else {
        let s = cursor_pos.saturating_sub(max_width.saturating_sub(1));
        (s, max_width.saturating_sub(1))
    };

    let visible: String = chars[start..len.min(start + max_width)].iter().collect();
    let mut spans = Vec::new();

    if display_cursor < visible.len() {
        let before: String = visible.chars().take(display_cursor).collect();
        let at: String = visible.chars().nth(display_cursor).map(|c| c.to_string()).unwrap_or_default();
        let after: String = visible.chars().skip(display_cursor + 1).collect();
        spans.push(Span::styled(before, Style::default().fg(Color::Rgb(210, 210, 225))));
        spans.push(Span::styled(at, Style::default().fg(Color::Black).bg(ACCENT)));
        spans.push(Span::styled(after, Style::default().fg(Color::Rgb(210, 210, 225))));
    } else {
        spans.push(Span::styled(visible, Style::default().fg(Color::Rgb(210, 210, 225))));
        spans.push(Span::styled(" ", Style::default().fg(Color::Black).bg(ACCENT)));
    }

    Line::from(spans)
}
