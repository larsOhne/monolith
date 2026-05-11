use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::Serialize;

use crate::{vault::Vault, vault_registry::VaultRegistry, worker, AppState};

// ─── Serialisable DTOs ───────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct VaultEntryDto {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub children: Vec<VaultEntryDto>,
}

fn entry_to_dto(e: &crate::vault::VaultEntry) -> VaultEntryDto {
    VaultEntryDto {
        path: e.path.display().to_string(),
        name: e.name.clone(),
        is_dir: e.is_dir,
        children: e.children.iter().map(entry_to_dto).collect(),
    }
}

#[derive(Serialize)]
pub struct DirItem {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

// ─── Vault registry ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_vaults() -> Vec<crate::vault_registry::VaultRecord> {
    VaultRegistry::load().vaults
}

#[tauri::command]
pub fn add_vault(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.is_dir() {
        return Err(format!("{path} is not a directory"));
    }
    let mut reg = VaultRegistry::load();
    reg.add(p);
    Ok(())
}

#[tauri::command]
pub fn remove_vault(path: String) {
    let mut reg = VaultRegistry::load();
    reg.remove(Path::new(&path));
}

// ─── File system ─────────────────────────────────────────────────────────────

/// List one directory level: dirs first, then files. Hidden entries excluded.
#[tauri::command]
pub fn browse_directory(path: String) -> Result<Vec<DirItem>, String> {
    let p = PathBuf::from(&path);
    let rd = std::fs::read_dir(&p)
        .map_err(|e| format!("read_dir failed: {e}"))?;
    let mut items: Vec<DirItem> = rd
        .flatten()
        .filter_map(|e| {
            let ep = e.path();
            let name = ep.file_name()?.to_string_lossy().into_owned();
            if name.starts_with('.') {
                return None;
            }
            let is_dir = ep.is_dir();
            Some(DirItem { name, path: ep.display().to_string(), is_dir })
        })
        .collect();
    items.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(items)
}

/// Read a file — must be inside the given vault root.
#[tauri::command]
pub fn read_vault_file(vault_root: String, path: String) -> Result<String, String> {
    let v = Vault::open(PathBuf::from(&vault_root));
    v.read_file(Path::new(&path)).map_err(|e| e.to_string())
}

/// Write a file — must be inside the given vault root.
#[tauri::command]
pub fn write_vault_file(vault_root: String, path: String, content: String) -> Result<(), String> {
    let v = Vault::open(PathBuf::from(&vault_root));
    v.write_file(Path::new(&path), &content).map_err(|e| e.to_string())
}

/// Full file tree for the given vault root.
#[tauri::command]
pub fn vault_tree(vault_root: String) -> Result<Vec<VaultEntryDto>, String> {
    let v = Vault::open(PathBuf::from(&vault_root));
    Ok(v.tree.iter().map(entry_to_dto).collect())
}

#[tauri::command]
pub fn create_note(vault_root: String, parent: String, name: String) -> Result<String, String> {
    let mut v = Vault::open(PathBuf::from(&vault_root));
    v.create_note(Path::new(&parent), &name)
        .map(|p| p.display().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_entry(vault_root: String, path: String) -> Result<(), String> {
    let mut v = Vault::open(PathBuf::from(&vault_root));
    v.delete_entry(Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_entry(vault_root: String, path: String, new_name: String) -> Result<String, String> {
    let mut v = Vault::open(PathBuf::from(&vault_root));
    v.rename_entry(Path::new(&path), &new_name)
        .map(|p| p.display().to_string())
        .map_err(|e| e.to_string())
}

// ─── Backend worker ───────────────────────────────────────────────────────────

/// Start the Python backend (idempotent). Returns the port it bound on.
#[tauri::command]
pub fn start_backend(state: tauri::State<AppState>) -> Result<u16, String> {
    // Already running?
    {
        let guard = state.worker.lock().unwrap();
        if let Some(ref h) = *guard {
            if let Some(port) = h.port() {
                return Ok(port);
            }
        }
    }

    let handle = worker::spawn_worker();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);

    loop {
        let port = handle.port();
        let failed = match &*handle.state.lock().unwrap() {
            worker::WorkerState::Failed(e) => Some(e.clone()),
            _ => None,
        };

        if let Some(port) = port {
            *state.worker.lock().unwrap() = Some(Arc::clone(&handle));
            return Ok(port);
        }
        if let Some(e) = failed {
            return Err(e);
        }
        if std::time::Instant::now() > deadline {
            return Err("Backend timed out after 30 s".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

#[tauri::command]
pub fn get_backend_port(state: tauri::State<AppState>) -> Option<u16> {
    state.worker.lock().unwrap().as_ref().and_then(|h| h.port())
}

#[tauri::command]
pub fn stop_backend(state: tauri::State<AppState>) {
    if let Some(h) = state.worker.lock().unwrap().take() {
        h.stop();
    }
}
