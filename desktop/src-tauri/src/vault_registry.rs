//! Persistent registry of known vault paths.
//!
//! Stored at `~/.config/monolith/vaults.json`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VaultRecord {
    /// Display name shown in the vault list (defaults to folder name).
    pub name: String,
    /// Absolute path to the vault root folder.
    pub path: PathBuf,
}

impl VaultRecord {
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        Self { name, path }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultRegistry {
    pub vaults: Vec<VaultRecord>,
}

impl VaultRegistry {
    // ── persistence ─────────────────────────────────────────────────────────

    fn config_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        let dir = PathBuf::from(home).join(".config").join("monolith");
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir.join("vaults.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        let Ok(bytes) = std::fs::read(&path) else {
            return Self::default();
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Ok(json) = serde_json::to_vec_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    // ── mutation ─────────────────────────────────────────────────────────────

    /// Add a vault to the registry if not already present. Returns `true` if added.
    pub fn add(&mut self, path: PathBuf) -> bool {
        if self.vaults.iter().any(|v| v.path == path) {
            return false;
        }
        self.vaults.push(VaultRecord::new(path));
        self.save();
        true
    }

    pub fn remove(&mut self, path: &Path) {
        self.vaults.retain(|v| v.path != path);
        self.save();
    }

    /// Rename the display name for a vault entry.
    pub fn rename(&mut self, path: &Path, name: String) {
        if let Some(v) = self.vaults.iter_mut().find(|v| v.path == path) {
            v.name = name;
            self.save();
        }
    }
}
