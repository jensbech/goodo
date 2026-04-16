use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};

use crate::app::{App, Mode};

const ZEBRA_DARK: Color = Color::Rgb(30, 30, 40);
const HIGHLIGHT_BG: Color = Color::Rgb(55, 55, 80);
const DIM: Color = Color::Rgb(100, 100, 110);
const ACCENT: Color = Color::Rgb(180, 180, 255);
const DETAIL_BG: Color = Color::Rgb(10, 8, 22);
const DONE_BG: Color = Color::Rgb(18, 34, 18);
const MAX_WIDTH: u16 = 120;

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
                Mode::Adding | Mode::AddingSubtask | Mode::Editing => 3,
                _ => 1,
            }),
        ])
        .split(content_area);

    draw_list(f, app, chunks[0]);

    match app.mode {
        Mode::Adding => draw_input(f, app, chunks[1], "Add todo"),
        Mode::AddingSubtask => draw_input(f, app, chunks[1], "Add subtask"),
        Mode::Editing => draw_input(f, app, chunks[1], "Edit todo"),
        _ => draw_footer(f, app, chunks[1]),
    }

    if let Mode::ConfirmDelete = app.mode {
        draw_confirm_delete(f, app, content_area);
    }
}

fn draw_list(f: &mut Frame, app: &App, area: Rect) {
    let flat = app.flat_view();
    let total = flat.len();
    let done_count = flat.iter().filter(|&&i| app.todos[i].done).count();

    let version = env!("CARGO_PKG_VERSION");
    let title = if total == 0 {
        Span::styled(
            format!(" goodo v{version} "),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            format!(" goodo v{version}  {done_count}/{total} ✓ "),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
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
        let centered = Rect {
            x: inner.x,
            y: inner.y + inner.height / 2,
            width: inner.width,
            height: 1,
        };
        f.render_widget(hint, centered);
        return;
    }

    let items: Vec<ListItem> = flat
        .iter()
        .enumerate()
        .map(|(display_i, &todo_i)| {
            let todo = &app.todos[todo_i];
            let is_selected = display_i == app.selected;
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

            let indent = if is_subtask { "  └ " } else { "" };

            let prefix = if is_selected {
                "▶ "
            } else if todo.done {
                "✓ "
            } else {
                "· "
            };

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
                Style::default()
                    .fg(Color::Rgb(75, 155, 75))
                    .add_modifier(Modifier::CROSSED_OUT)
                    .bg(bg)
            } else if is_subtask {
                Style::default().fg(Color::Rgb(185, 185, 205)).bg(bg)
            } else {
                Style::default().fg(Color::Rgb(210, 210, 225)).bg(bg)
            };

            let indent_style = Style::default().fg(Color::Rgb(70, 70, 95)).bg(bg);
            let prefix_style = Style::default().fg(prefix_color).bg(bg);

            let mut spans = Vec::new();
            if is_subtask {
                spans.push(Span::styled(indent, indent_style));
            }
            spans.push(Span::styled(prefix, prefix_style));
            spans.push(Span::styled(todo.text.clone(), text_style));

            if !is_subtask && todo.parent_id.is_none() {
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
                    let badge = format!("  ({done_sub}/{total_sub})");
                    spans.push(Span::styled(badge, Style::default().fg(badge_color).bg(bg)));
                }
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, inner);
}

fn draw_input(f: &mut Frame, app: &App, area: Rect, title: &str) {
    let visible = visible_input(&app.input, app.cursor_pos, area.width.saturating_sub(4) as usize);

    let block = Block::default()
        .title(Span::styled(
            format!(" {} ", title),
            Style::default().fg(ACCENT),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(DETAIL_BG));

    let para = Paragraph::new(visible).block(block);
    f.render_widget(para, area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let has_todos = !app.flat_view().is_empty();
    let selected_is_top_level = app.selected_todo_idx()
        .map(|i| app.todos[i].parent_id.is_none())
        .unwrap_or(false);

    let s = Style::default();
    let key = |k: &'static str| Span::styled(k, s.fg(ACCENT).add_modifier(Modifier::BOLD));
    let label = |l: &'static str| Span::styled(l, s.fg(DIM));

    let hints: Vec<Span> = if !has_todos {
        vec![key("[a]"), label("dd  "), key("[q]"), label("uit")]
    } else if selected_is_top_level {
        vec![
            key("[a]"), label("dd  "),
            key("[A]"), label(" subtask  "),
            key("[e]"), label("dit  "),
            key("[J/K]"), label(" move  "),
            key("[tab]"), label(" indent  "),
            key("[space]"), label(" toggle  "),
            key("[x]"), label("/"), key("[d]"), label("el  "),
            key("[u]"), label("ndo  "),
            key("[r]"), label("edo  "),
            key("[q]"), label("uit"),
        ]
    } else {
        vec![
            key("[a]"), label("dd  "),
            key("[e]"), label("dit  "),
            key("[J/K]"), label(" move  "),
            key("[tab]"), label(" dedent  "),
            key("[space]"), label(" toggle  "),
            key("[x]"), label("/"), key("[d]"), label("el  "),
            key("[u]"), label("ndo  "),
            key("[r]"), label("edo  "),
            key("[q]"), label("uit"),
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
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(200, 80, 80)))
        .style(Style::default().bg(DETAIL_BG));

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    let para = Paragraph::new(Span::styled(
        text,
        Style::default().fg(Color::Rgb(220, 200, 200)),
    ))
    .alignment(Alignment::Center);
    f.render_widget(para, inner);
}

fn visible_input(input: &str, cursor_pos: usize, max_width: usize) -> Line<'static> {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();

    let (start, display_cursor) = if cursor_pos <= max_width.saturating_sub(1) {
        (0, cursor_pos)
    } else {
        let start = cursor_pos.saturating_sub(max_width.saturating_sub(1));
        (start, max_width.saturating_sub(1))
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
