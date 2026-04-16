use std::fs;
use std::path::PathBuf;

use crate::app::Todo;

fn data_path() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("goodo");
    path.push("todos.json");
    path
}

pub fn load() -> Vec<Todo> {
    let path = data_path();
    if !path.exists() {
        return Vec::new();
    }
    let contents = fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str(&contents).unwrap_or_default()
}

pub fn save(todos: &[Todo]) {
    let path = data_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(todos) {
        let _ = fs::write(&path, json);
    }
}
