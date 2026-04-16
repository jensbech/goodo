use serde::{Deserialize, Serialize};

use crate::storage;

#[derive(Serialize, Deserialize, Clone)]
pub struct Todo {
    pub id: u64,
    pub text: String,
    pub done: bool,
    pub parent_id: Option<u64>,
}

pub enum Mode {
    Normal,
    Adding,
    AddingSubtask,
    Editing,
    ConfirmDelete,
}

pub struct App {
    pub todos: Vec<Todo>,
    pub selected: usize,
    pub mode: Mode,
    pub input: String,
    pub cursor_pos: usize,
    pub next_id: u64,
    pub adding_parent_id: Option<u64>,
    undo_stack: Vec<(Vec<Todo>, usize)>,
    redo_stack: Vec<(Vec<Todo>, usize)>,
}

impl App {
    pub fn new() -> Self {
        let todos = storage::load();
        let next_id = todos.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        Self {
            todos,
            selected: 0,
            mode: Mode::Normal,
            input: String::new(),
            cursor_pos: 0,
            next_id,
            adding_parent_id: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    fn snapshot(&mut self) {
        self.undo_stack.push((self.todos.clone(), self.selected));
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some((todos, selected)) = self.undo_stack.pop() {
            self.redo_stack.push((self.todos.clone(), self.selected));
            self.todos = todos;
            self.selected = selected;
            storage::save(&self.todos);
        }
    }

    pub fn redo(&mut self) {
        if let Some((todos, selected)) = self.redo_stack.pop() {
            self.undo_stack.push((self.todos.clone(), self.selected));
            self.todos = todos;
            self.selected = selected;
            storage::save(&self.todos);
        }
    }

    pub fn flat_view(&self) -> Vec<usize> {
        let mut result = Vec::new();
        for (i, todo) in self.todos.iter().enumerate() {
            if todo.parent_id.is_none() {
                result.push(i);
                for (j, child) in self.todos.iter().enumerate() {
                    if child.parent_id == Some(todo.id) {
                        result.push(j);
                    }
                }
            }
        }
        result
    }

    pub fn selected_todo_idx(&self) -> Option<usize> {
        self.flat_view().get(self.selected).copied()
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let len = self.flat_view().len();
        if len > 0 && self.selected < len - 1 {
            self.selected += 1;
        }
    }

    pub fn move_item_up(&mut self) {
        let flat = self.flat_view();
        if self.selected == 0 || flat.is_empty() { return; }
        self.snapshot();

        let cur_idx = flat[self.selected];
        let cur_parent = self.todos[cur_idx].parent_id;

        if cur_parent.is_some() {
            let prev_sib_pos = flat[..self.selected]
                .iter()
                .rposition(|&i| self.todos[i].parent_id == cur_parent);
            let Some(prev_pos) = prev_sib_pos else { return };
            let mut new_flat = flat.clone();
            new_flat.swap(self.selected, prev_pos);
            self.todos = new_flat.iter().map(|&i| self.todos[i].clone()).collect();
            self.selected = prev_pos;
        } else {
            let prev_top_pos = flat[..self.selected]
                .iter()
                .rposition(|&i| self.todos[i].parent_id.is_none());
            let Some(prev_top) = prev_top_pos else { return };

            let lower_end = flat[self.selected..]
                .iter()
                .skip(1)
                .position(|&i| self.todos[i].parent_id.is_none())
                .map(|p| self.selected + 1 + p)
                .unwrap_or(flat.len());

            let mut new_flat = flat[..prev_top].to_vec();
            new_flat.extend_from_slice(&flat[self.selected..lower_end]);
            new_flat.extend_from_slice(&flat[prev_top..self.selected]);
            new_flat.extend_from_slice(&flat[lower_end..]);
            self.todos = new_flat.iter().map(|&i| self.todos[i].clone()).collect();
            self.selected = prev_top;
        }

        storage::save(&self.todos);
    }

    pub fn move_item_down(&mut self) {
        let flat = self.flat_view();
        if flat.is_empty() || self.selected >= flat.len() - 1 { return; }
        self.snapshot();

        let cur_idx = flat[self.selected];
        let cur_parent = self.todos[cur_idx].parent_id;

        if cur_parent.is_some() {
            let next_sib_pos = flat[self.selected + 1..]
                .iter()
                .position(|&i| self.todos[i].parent_id == cur_parent)
                .map(|p| self.selected + 1 + p);
            let Some(next_pos) = next_sib_pos else { return };
            let mut new_flat = flat.clone();
            new_flat.swap(self.selected, next_pos);
            self.todos = new_flat.iter().map(|&i| self.todos[i].clone()).collect();
            self.selected = next_pos;
        } else {
            let cur_end = flat[self.selected..]
                .iter()
                .skip(1)
                .position(|&i| self.todos[i].parent_id.is_none())
                .map(|p| self.selected + 1 + p)
                .unwrap_or(flat.len());

            if cur_end >= flat.len() { return; }

            let next_end = flat[cur_end..]
                .iter()
                .skip(1)
                .position(|&i| self.todos[i].parent_id.is_none())
                .map(|p| cur_end + 1 + p)
                .unwrap_or(flat.len());

            let mut new_flat = flat[..self.selected].to_vec();
            new_flat.extend_from_slice(&flat[cur_end..next_end]);
            new_flat.extend_from_slice(&flat[self.selected..cur_end]);
            new_flat.extend_from_slice(&flat[next_end..]);
            let new_sel = self.selected + (next_end - cur_end);
            self.todos = new_flat.iter().map(|&i| self.todos[i].clone()).collect();
            self.selected = new_sel;
        }

        storage::save(&self.todos);
    }

    pub fn indent(&mut self) {
        let flat = self.flat_view();
        let Some(&cur_idx) = flat.get(self.selected) else { return };
        if self.todos[cur_idx].parent_id.is_some() { return; }
        let has_children = self.todos.iter().any(|t| t.parent_id == Some(self.todos[cur_idx].id));
        if has_children { return; }

        let prev_top_idx = flat[..self.selected]
            .iter()
            .rfind(|&&i| self.todos[i].parent_id.is_none())
            .copied();

        if let Some(parent_idx) = prev_top_idx {
            self.snapshot();
            let parent_id = self.todos[parent_idx].id;
            self.todos[cur_idx].parent_id = Some(parent_id);
            storage::save(&self.todos);
        }
    }

    pub fn unindent(&mut self) {
        let flat = self.flat_view();
        let Some(&cur_idx) = flat.get(self.selected) else { return };
        if self.todos[cur_idx].parent_id.is_none() { return; }
        self.snapshot();
        self.todos[cur_idx].parent_id = None;
        storage::save(&self.todos);
    }

    pub fn start_adding(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
        self.adding_parent_id = None;
        self.mode = Mode::Adding;
    }

    pub fn start_adding_subtask(&mut self) {
        let Some(idx) = self.selected_todo_idx() else { return };
        let todo = &self.todos[idx];
        self.adding_parent_id = Some(todo.parent_id.unwrap_or(todo.id));
        self.input.clear();
        self.cursor_pos = 0;
        self.mode = Mode::AddingSubtask;
    }

    pub fn start_editing(&mut self) {
        let Some(idx) = self.selected_todo_idx() else { return };
        self.input = self.todos[idx].text.clone();
        self.cursor_pos = self.input.chars().count();
        self.mode = Mode::Editing;
    }

    pub fn confirm_add(&mut self) {
        let text = self.input.trim().to_string();
        if !text.is_empty() {
            self.snapshot();
            let id = self.next_id;
            self.next_id += 1;
            let parent_id = self.adding_parent_id;
            self.todos.push(Todo { id, text, done: false, parent_id });
            let fv = self.flat_view();
            if let Some(pos) = fv.iter().position(|&i| self.todos[i].id == id) {
                self.selected = pos;
            }
            storage::save(&self.todos);
        }
        self.mode = Mode::Normal;
        self.input.clear();
        self.cursor_pos = 0;
        self.adding_parent_id = None;
    }

    pub fn confirm_edit(&mut self) {
        let text = self.input.trim().to_string();
        if !text.is_empty() {
            if let Some(idx) = self.selected_todo_idx() {
                self.snapshot();
                self.todos[idx].text = text;
                storage::save(&self.todos);
            }
        }
        self.mode = Mode::Normal;
        self.input.clear();
        self.cursor_pos = 0;
    }

    pub fn cancel_input(&mut self) {
        self.mode = Mode::Normal;
        self.input.clear();
        self.cursor_pos = 0;
        self.adding_parent_id = None;
    }

    pub fn toggle_done(&mut self) {
        let Some(idx) = self.selected_todo_idx() else { return };
        self.snapshot();
        let new_done = !self.todos[idx].done;
        let id = self.todos[idx].id;
        self.todos[idx].done = new_done;
        for child in self.todos.iter_mut() {
            if child.parent_id == Some(id) {
                child.done = new_done;
            }
        }
        storage::save(&self.todos);
    }

    pub fn start_delete(&mut self) {
        if self.selected_todo_idx().is_some() {
            self.mode = Mode::ConfirmDelete;
        }
    }

    pub fn quick_delete(&mut self) {
        let Some(idx) = self.selected_todo_idx() else { return };
        let id = self.todos[idx].id;
        let has_children = self.todos.iter().any(|t| t.parent_id == Some(id));
        if has_children {
            self.mode = Mode::ConfirmDelete;
        } else {
            self.snapshot();
            self.todos.retain(|t| t.id != id);
            let fv_len = self.flat_view().len();
            if fv_len == 0 {
                self.selected = 0;
            } else if self.selected >= fv_len {
                self.selected = fv_len - 1;
            }
            storage::save(&self.todos);
        }
    }

    pub fn confirm_delete(&mut self) {
        if let Some(idx) = self.selected_todo_idx() {
            self.snapshot();
            let id = self.todos[idx].id;
            self.todos.retain(|t| t.id != id && t.parent_id != Some(id));
            let fv_len = self.flat_view().len();
            if fv_len == 0 {
                self.selected = 0;
            } else if self.selected >= fv_len {
                self.selected = fv_len - 1;
            }
            storage::save(&self.todos);
        }
        self.mode = Mode::Normal;
    }

    pub fn jump_top(&mut self) {
        self.selected = 0;
    }

    pub fn jump_bottom(&mut self) {
        let len = self.flat_view().len();
        if len > 0 {
            self.selected = len - 1;
        }
    }

    pub fn input_insert_char(&mut self, c: char) {
        let byte_pos = self.input.char_indices()
            .nth(self.cursor_pos)
            .map(|(i, _)| i)
            .unwrap_or(self.input.len());
        self.input.insert(byte_pos, c);
        self.cursor_pos += 1;
    }

    pub fn input_backspace(&mut self) {
        if self.cursor_pos == 0 { return; }
        let byte_pos = self.input.char_indices()
            .nth(self.cursor_pos - 1)
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.input.remove(byte_pos);
        self.cursor_pos -= 1;
    }

    pub fn input_move_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn input_move_right(&mut self) {
        if self.cursor_pos < self.input.chars().count() {
            self.cursor_pos += 1;
        }
    }
}
