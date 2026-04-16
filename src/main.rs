mod app;
mod storage;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use app::{App, Mode};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

fn run<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            match app.mode {
                Mode::Normal => match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('j') | KeyCode::Down => app.move_down(),
                    KeyCode::Char('k') | KeyCode::Up => app.move_up(),
                    KeyCode::Char('J') => app.move_item_down(),
                    KeyCode::Char('K') => app.move_item_up(),
                    KeyCode::Char('g') => app.jump_top(),
                    KeyCode::Char('G') => app.jump_bottom(),
                    KeyCode::Tab => app.indent(),
                    KeyCode::BackTab => app.unindent(),
                    KeyCode::Char('a') => app.start_adding(),
                    KeyCode::Char('A') => app.start_adding_subtask(),
                    KeyCode::Char('n') => app.start_adding_section(),
                    KeyCode::Char('e') => {
                        if app.selected_section_idx().is_some() {
                            app.start_editing_section();
                        } else {
                            app.start_editing();
                        }
                    }
                    KeyCode::Char(' ') | KeyCode::Enter => app.toggle_done(),
                    KeyCode::Char('x') => app.quick_delete(),
                    KeyCode::Char('d') => app.start_delete(),
                    KeyCode::Char('u') => app.undo(),
                    KeyCode::Char('r') => app.redo(),
                    _ => {}
                },
                Mode::Adding | Mode::AddingSubtask | Mode::AddingSection => match key.code {
                    KeyCode::Enter => {
                        match app.mode {
                            Mode::AddingSection => app.confirm_add_section(),
                            _ => app.confirm_add(),
                        }
                    }
                    KeyCode::Esc => app.cancel_input(),
                    KeyCode::Backspace => app.input_backspace(),
                    KeyCode::Left => app.input_move_left(),
                    KeyCode::Right => app.input_move_right(),
                    KeyCode::Char(c) => app.input_insert_char(c),
                    _ => {}
                },
                Mode::Editing | Mode::EditingSection => match key.code {
                    KeyCode::Enter => {
                        match app.mode {
                            Mode::EditingSection => app.confirm_edit_section(),
                            _ => app.confirm_edit(),
                        }
                    }
                    KeyCode::Esc => app.cancel_input(),
                    KeyCode::Backspace => app.input_backspace(),
                    KeyCode::Left => app.input_move_left(),
                    KeyCode::Right => app.input_move_right(),
                    KeyCode::Char(c) => app.input_insert_char(c),
                    _ => {}
                },
                Mode::ConfirmDelete => match key.code {
                    KeyCode::Char('y') | KeyCode::Enter => app.confirm_delete(),
                    KeyCode::Char('n') | KeyCode::Esc => app.mode = Mode::Normal,
                    _ => {}
                },
                Mode::ConfirmDeleteSection => match key.code {
                    KeyCode::Char('y') | KeyCode::Enter => app.confirm_delete_section(),
                    KeyCode::Char('n') | KeyCode::Esc => app.mode = Mode::Normal,
                    _ => {}
                },
            }
        }
    }

    Ok(())
}
