use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::app::{Section, Todo};

#[derive(Serialize, Deserialize)]
struct StoreFile {
    sections: Vec<Section>,
    todos: Vec<Todo>,
}

fn data_path() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("goodo");
    path.push("todos.json");
    path
}

pub fn load() -> (Vec<Section>, Vec<Todo>) {
    let path = data_path();
    if !path.exists() {
        return default_store();
    }
    let contents = fs::read_to_string(&path).unwrap_or_default();

    if let Ok(store) = serde_json::from_str::<StoreFile>(&contents) {
        if !store.sections.is_empty() {
            return (store.sections, store.todos);
        }
    }

    if let Ok(todos) = serde_json::from_str::<Vec<Todo>>(&contents) {
        return (vec![Section { id: 1, name: "General".to_string() }], todos);
    }

    default_store()
}

fn default_store() -> (Vec<Section>, Vec<Todo>) {
    (vec![Section { id: 1, name: "General".to_string() }], Vec::new())
}

pub fn save(sections: &[Section], todos: &[Todo]) {
    let path = data_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let store = StoreFile {
        sections: sections.to_vec(),
        todos: todos.to_vec(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&store) {
        let _ = fs::write(&path, json);
    }
}
