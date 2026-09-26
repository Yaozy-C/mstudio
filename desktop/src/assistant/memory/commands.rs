use super::{Memory, MemoryBackend, SqliteMemory, now};
use crate::database::Store;
use tauri::State;
#[tauri::command]
pub fn project_memory(store: State<Store>, project_id: String) -> Result<Memory, String> {
    SqliteMemory(&store.db.lock().unwrap())
        .recall(&project_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn save_project_memory(
    store: State<Store>,
    project_id: String,
    mut memory: Memory,
) -> Result<Memory, String> {
    let db = store.db.lock().unwrap();
    let backend = SqliteMemory(&db);
    let before = backend.recall(&project_id).map_err(|e| e.to_string())?;
    for entry in &mut memory.entries {
        if let Some(old) = before
            .entries
            .iter()
            .find(|e| e.id == entry.id && e.title == entry.title && e.content == entry.content)
        {
            *entry = old.clone();
        } else {
            entry.source = "用户在项目设置中编辑".into();
            entry.turn_id = None;
            entry.updated = now();
        }
    }
    backend
        .replace(&project_id, memory)
        .map_err(|e| e.to_string())
}
