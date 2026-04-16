use serde::{Deserialize, Serialize};

use crate::storage;

#[derive(Serialize, Deserialize, Clone)]
pub struct Section {
    pub id: u64,
    pub name: String,
}

fn default_section_id() -> u64 { 1 }

#[derive(Serialize, Deserialize, Clone)]
pub struct Todo {
    pub id: u64,
    pub text: String,
    pub done: bool,
    pub parent_id: Option<u64>,
    #[serde(default = "default_section_id")]
    pub section_id: u64,
}

#[derive(Clone)]
pub enum DisplayItem {
    SectionHeading(usize),
    Todo(usize),
}

pub enum Mode {
    Normal,
    Adding,
    AddingSubtask,
    AddingSection,
    Editing,
    EditingSection,
    ConfirmDelete,
    ConfirmDeleteSection,
}

struct Snapshot {
    sections: Vec<Section>,
    todos: Vec<Todo>,
    selected: usize,
}

pub struct App {
    pub sections: Vec<Section>,
    pub todos: Vec<Todo>,
    pub selected: usize,
    pub mode: Mode,
    pub input: String,
    pub cursor_pos: usize,
    pub next_id: u64,
    pub next_section_id: u64,
    pub adding_parent_id: Option<u64>,
    pub adding_section_id: u64,
    undo_stack: Vec<Snapshot>,
    redo_stack: Vec<Snapshot>,
}

impl App {
    pub fn new() -> Self {
        let (sections, todos) = storage::load();
        let next_id = todos.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let next_section_id = sections.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        let default_sid = sections.first().map(|s| s.id).unwrap_or(1);
        Self {
            sections,
            todos,
            selected: 0,
            mode: Mode::Normal,
            input: String::new(),
            cursor_pos: 0,
            next_id,
            next_section_id,
            adding_parent_id: None,
            adding_section_id: default_sid,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    fn save(&self) {
        storage::save(&self.sections, &self.todos);
    }

    fn snapshot(&mut self) {
        self.undo_stack.push(Snapshot {
            sections: self.sections.clone(),
            todos: self.todos.clone(),
            selected: self.selected,
        });
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(snap) = self.undo_stack.pop() {
            self.redo_stack.push(Snapshot {
                sections: self.sections.clone(),
                todos: self.todos.clone(),
                selected: self.selected,
            });
            self.sections = snap.sections;
            self.todos = snap.todos;
            self.selected = snap.selected;
            self.save();
        }
    }

    pub fn redo(&mut self) {
        if let Some(snap) = self.redo_stack.pop() {
            self.undo_stack.push(Snapshot {
                sections: self.sections.clone(),
                todos: self.todos.clone(),
                selected: self.selected,
            });
            self.sections = snap.sections;
            self.todos = snap.todos;
            self.selected = snap.selected;
            self.save();
        }
    }

    pub fn flat_view(&self) -> Vec<DisplayItem> {
        let mut result = Vec::new();
        for (si, section) in self.sections.iter().enumerate() {
            result.push(DisplayItem::SectionHeading(si));
            for (ti, todo) in self.todos.iter().enumerate() {
                if todo.section_id == section.id && todo.parent_id.is_none() {
                    result.push(DisplayItem::Todo(ti));
                    for (ci, child) in self.todos.iter().enumerate() {
                        if child.parent_id == Some(todo.id) {
                            result.push(DisplayItem::Todo(ci));
                        }
                    }
                }
            }
        }
        result
    }

    fn all_todos_display_order(&self) -> Vec<usize> {
        self.flat_view().into_iter().filter_map(|item| {
            if let DisplayItem::Todo(i) = item { Some(i) } else { None }
        }).collect()
    }

    pub fn selected_todo_idx(&self) -> Option<usize> {
        match self.flat_view().get(self.selected)? {
            DisplayItem::Todo(i) => Some(*i),
            _ => None,
        }
    }

    pub fn selected_section_idx(&self) -> Option<usize> {
        match self.flat_view().get(self.selected)? {
            DisplayItem::SectionHeading(i) => Some(*i),
            _ => None,
        }
    }

    pub fn current_section_id(&self) -> u64 {
        let flat = self.flat_view();
        if let Some(item) = flat.get(self.selected) {
            match item {
                DisplayItem::SectionHeading(si) => self.sections[*si].id,
                DisplayItem::Todo(ti) => self.todos[*ti].section_id,
            }
        } else {
            self.sections.first().map(|s| s.id).unwrap_or(1)
        }
    }

    fn find_todo_in_flat(&self, todo_id: u64) -> Option<usize> {
        let flat = self.flat_view();
        flat.iter().position(|item| {
            if let DisplayItem::Todo(i) = item { self.todos[*i].id == todo_id } else { false }
        })
    }

    fn find_section_in_flat(&self, section_id: u64) -> Option<usize> {
        let flat = self.flat_view();
        flat.iter().position(|item| {
            if let DisplayItem::SectionHeading(si) = item { self.sections[*si].id == section_id } else { false }
        })
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

        match flat[self.selected].clone() {
            DisplayItem::SectionHeading(si) => {
                if si == 0 { return; }
                self.snapshot();
                let section_id = self.sections[si].id;
                self.sections.swap(si - 1, si);
                self.selected = self.find_section_in_flat(section_id).unwrap_or(self.selected);
                self.save();
            }
            DisplayItem::Todo(cur_idx) => {
                let all = self.all_todos_display_order();
                let cur_pos = all.iter().position(|&i| i == cur_idx).unwrap();
                let cur_id = self.todos[cur_idx].id;
                let cur_section = self.todos[cur_idx].section_id;

                if self.todos[cur_idx].parent_id.is_some() {
                    let parent_id = self.todos[cur_idx].parent_id;
                    let prev = all[..cur_pos].iter().rposition(|&i| self.todos[i].parent_id == parent_id);
                    let Some(prev_pos) = prev else { return };
                    self.snapshot();
                    let mut new_all = all.clone();
                    new_all.swap(cur_pos, prev_pos);
                    self.todos = new_all.iter().map(|&i| self.todos[i].clone()).collect();
                } else {
                    let prev_top = all[..cur_pos].iter().rposition(|&i| {
                        self.todos[i].parent_id.is_none() && self.todos[i].section_id == cur_section
                    });
                    let Some(prev_top_pos) = prev_top else { return };
                    self.snapshot();

                    let section_end = all[cur_pos..].iter()
                        .position(|&i| self.todos[i].section_id != cur_section)
                        .map(|p| cur_pos + p)
                        .unwrap_or(all.len());

                    let lower_end = all[cur_pos..section_end].iter().skip(1)
                        .position(|&i| self.todos[i].parent_id.is_none())
                        .map(|p| cur_pos + 1 + p)
                        .unwrap_or(section_end);

                    let mut new_all = all[..prev_top_pos].to_vec();
                    new_all.extend_from_slice(&all[cur_pos..lower_end]);
                    new_all.extend_from_slice(&all[prev_top_pos..cur_pos]);
                    new_all.extend_from_slice(&all[lower_end..]);
                    self.todos = new_all.iter().map(|&i| self.todos[i].clone()).collect();
                }

                self.selected = self.find_todo_in_flat(cur_id).unwrap_or(self.selected);
                self.save();
            }
        }
    }

    pub fn move_item_down(&mut self) {
        let flat = self.flat_view();
        if flat.is_empty() || self.selected >= flat.len() - 1 { return; }

        match flat[self.selected].clone() {
            DisplayItem::SectionHeading(si) => {
                if si >= self.sections.len() - 1 { return; }
                self.snapshot();
                let section_id = self.sections[si].id;
                self.sections.swap(si, si + 1);
                self.selected = self.find_section_in_flat(section_id).unwrap_or(self.selected);
                self.save();
            }
            DisplayItem::Todo(cur_idx) => {
                let all = self.all_todos_display_order();
                let cur_pos = all.iter().position(|&i| i == cur_idx).unwrap();
                let cur_id = self.todos[cur_idx].id;
                let cur_section = self.todos[cur_idx].section_id;

                if self.todos[cur_idx].parent_id.is_some() {
                    let parent_id = self.todos[cur_idx].parent_id;
                    let next = all[cur_pos + 1..].iter()
                        .position(|&i| self.todos[i].parent_id == parent_id)
                        .map(|p| cur_pos + 1 + p);
                    let Some(next_pos) = next else { return };
                    self.snapshot();
                    let mut new_all = all.clone();
                    new_all.swap(cur_pos, next_pos);
                    self.todos = new_all.iter().map(|&i| self.todos[i].clone()).collect();
                } else {
                    let section_end = all[cur_pos..].iter()
                        .position(|&i| self.todos[i].section_id != cur_section)
                        .map(|p| cur_pos + p)
                        .unwrap_or(all.len());

                    let cur_end = all[cur_pos..section_end].iter().skip(1)
                        .position(|&i| self.todos[i].parent_id.is_none())
                        .map(|p| cur_pos + 1 + p)
                        .unwrap_or(section_end);

                    if cur_end >= section_end { return; }

                    let next_end = all[cur_end..section_end].iter().skip(1)
                        .position(|&i| self.todos[i].parent_id.is_none())
                        .map(|p| cur_end + 1 + p)
                        .unwrap_or(section_end);

                    self.snapshot();
                    let mut new_all = all[..cur_pos].to_vec();
                    new_all.extend_from_slice(&all[cur_end..next_end]);
                    new_all.extend_from_slice(&all[cur_pos..cur_end]);
                    new_all.extend_from_slice(&all[next_end..]);
                    self.todos = new_all.iter().map(|&i| self.todos[i].clone()).collect();
                }

                self.selected = self.find_todo_in_flat(cur_id).unwrap_or(self.selected);
                self.save();
            }
        }
    }

    pub fn indent(&mut self) {
        let flat = self.flat_view();
        let Some(DisplayItem::Todo(cur_idx)) = flat.get(self.selected).cloned() else { return };
        if self.todos[cur_idx].parent_id.is_some() { return; }
        if self.todos.iter().any(|t| t.parent_id == Some(self.todos[cur_idx].id)) { return; }

        let cur_section = self.todos[cur_idx].section_id;
        let prev_top = flat[..self.selected].iter().rev().find_map(|item| {
            if let DisplayItem::Todo(i) = item {
                if self.todos[*i].parent_id.is_none() && self.todos[*i].section_id == cur_section {
                    return Some(*i);
                }
            }
            None
        });

        if let Some(parent_idx) = prev_top {
            self.snapshot();
            let parent_id = self.todos[parent_idx].id;
            self.todos[cur_idx].parent_id = Some(parent_id);
            self.save();
        }
    }

    pub fn unindent(&mut self) {
        let flat = self.flat_view();
        let Some(DisplayItem::Todo(cur_idx)) = flat.get(self.selected).cloned() else { return };
        if self.todos[cur_idx].parent_id.is_none() { return; }
        self.snapshot();
        self.todos[cur_idx].parent_id = None;
        self.save();
    }

    pub fn start_adding(&mut self) {
        self.adding_section_id = self.current_section_id();
        self.adding_parent_id = None;
        self.input.clear();
        self.cursor_pos = 0;
        self.mode = Mode::Adding;
    }

    pub fn start_adding_subtask(&mut self) {
        let Some(idx) = self.selected_todo_idx() else { return };
        let todo = &self.todos[idx];
        self.adding_section_id = todo.section_id;
        self.adding_parent_id = Some(todo.parent_id.unwrap_or(todo.id));
        self.input.clear();
        self.cursor_pos = 0;
        self.mode = Mode::AddingSubtask;
    }

    pub fn start_adding_section(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
        self.mode = Mode::AddingSection;
    }

    pub fn start_editing(&mut self) {
        let Some(idx) = self.selected_todo_idx() else { return };
        self.input = self.todos[idx].text.clone();
        self.cursor_pos = self.input.chars().count();
        self.mode = Mode::Editing;
    }

    pub fn start_editing_section(&mut self) {
        let Some(si) = self.selected_section_idx() else { return };
        self.input = self.sections[si].name.clone();
        self.cursor_pos = self.input.chars().count();
        self.mode = Mode::EditingSection;
    }

    pub fn confirm_add(&mut self) {
        let text = self.input.trim().to_string();
        if !text.is_empty() {
            self.snapshot();
            let id = self.next_id;
            self.next_id += 1;
            self.todos.push(Todo {
                id,
                text,
                done: false,
                parent_id: self.adding_parent_id,
                section_id: self.adding_section_id,
            });
            self.selected = self.find_todo_in_flat(id).unwrap_or(self.selected);
            self.save();
        }
        self.mode = Mode::Normal;
        self.input.clear();
        self.cursor_pos = 0;
        self.adding_parent_id = None;
    }

    pub fn confirm_add_section(&mut self) {
        let name = self.input.trim().to_string();
        if !name.is_empty() {
            self.snapshot();
            let id = self.next_section_id;
            self.next_section_id += 1;
            let cur_si = self.sections.iter()
                .position(|s| s.id == self.current_section_id())
                .unwrap_or(self.sections.len().saturating_sub(1));
            self.sections.insert(cur_si + 1, Section { id, name });
            self.selected = self.find_section_in_flat(id).unwrap_or(self.selected);
            self.save();
        }
        self.mode = Mode::Normal;
        self.input.clear();
        self.cursor_pos = 0;
    }

    pub fn confirm_edit(&mut self) {
        let text = self.input.trim().to_string();
        if !text.is_empty() {
            if let Some(idx) = self.selected_todo_idx() {
                self.snapshot();
                self.todos[idx].text = text;
                self.save();
            }
        }
        self.mode = Mode::Normal;
        self.input.clear();
        self.cursor_pos = 0;
    }

    pub fn confirm_edit_section(&mut self) {
        let name = self.input.trim().to_string();
        if !name.is_empty() {
            if let Some(si) = self.selected_section_idx() {
                self.snapshot();
                self.sections[si].name = name;
                self.save();
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
        self.save();
    }

    pub fn start_delete(&mut self) {
        if self.selected_section_idx().is_some() {
            self.start_delete_section();
        } else if self.selected_todo_idx().is_some() {
            self.mode = Mode::ConfirmDelete;
        }
    }

    pub fn quick_delete(&mut self) {
        if self.selected_section_idx().is_some() {
            self.start_delete_section();
            return;
        }
        let Some(idx) = self.selected_todo_idx() else { return };
        let id = self.todos[idx].id;
        let has_children = self.todos.iter().any(|t| t.parent_id == Some(id));
        if has_children {
            self.mode = Mode::ConfirmDelete;
        } else {
            self.snapshot();
            self.todos.retain(|t| t.id != id);
            let fv_len = self.flat_view().len();
            if self.selected >= fv_len && self.selected > 0 {
                self.selected = fv_len.saturating_sub(1);
            }
            self.save();
        }
    }

    pub fn confirm_delete(&mut self) {
        if let Some(idx) = self.selected_todo_idx() {
            self.snapshot();
            let id = self.todos[idx].id;
            self.todos.retain(|t| t.id != id && t.parent_id != Some(id));
            let fv_len = self.flat_view().len();
            if self.selected >= fv_len && self.selected > 0 {
                self.selected = fv_len.saturating_sub(1);
            }
            self.save();
        }
        self.mode = Mode::Normal;
    }

    pub fn start_delete_section(&mut self) {
        if self.sections.len() <= 1 { return; }
        if self.selected_section_idx().is_some() {
            self.mode = Mode::ConfirmDeleteSection;
        }
    }

    pub fn confirm_delete_section(&mut self) {
        if let Some(si) = self.selected_section_idx() {
            self.snapshot();
            let section_id = self.sections[si].id;
            self.todos.retain(|t| t.section_id != section_id);
            self.sections.remove(si);
            let fv_len = self.flat_view().len();
            if self.selected >= fv_len && self.selected > 0 {
                self.selected = fv_len.saturating_sub(1);
            }
            self.save();
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
        if self.cursor_pos > 0 { self.cursor_pos -= 1; }
    }

    pub fn input_move_right(&mut self) {
        if self.cursor_pos < self.input.chars().count() { self.cursor_pos += 1; }
    }
}
