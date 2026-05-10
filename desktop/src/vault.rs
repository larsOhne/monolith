//! Vault: a folder of Markdown files on disk.

use std::{
    fs,
    path::{Path, PathBuf},
};

/// A node in the file tree.
#[derive(Debug, Clone)]
pub struct VaultEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub children: Vec<VaultEntry>,
}

impl VaultEntry {
    fn scan(path: &Path) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let is_dir = path.is_dir();
        let mut children = Vec::new();
        if is_dir {
            if let Ok(entries) = fs::read_dir(path) {
                let mut paths: Vec<PathBuf> = entries
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .map(|n| !n.starts_with('.'))
                            .unwrap_or(false)
                    })
                    .collect();
                paths.sort_by(|a, b| {
                    let ad = a.is_dir();
                    let bd = b.is_dir();
                    bd.cmp(&ad).then(a.file_name().cmp(&b.file_name()))
                });
                children = paths.iter().map(|p| VaultEntry::scan(p)).collect();
            }
        }
        VaultEntry { path: path.to_path_buf(), name, is_dir, children }
    }
}

/// Top-level vault state.
pub struct Vault {
    pub root: PathBuf,
    pub tree: Vec<VaultEntry>,
}

impl Vault {
    pub fn open(root: PathBuf) -> Self {
        let tree = Self::scan_children(&root);
        Vault { root, tree }
    }

    pub fn refresh(&mut self) {
        self.tree = Self::scan_children(&self.root);
    }

    fn scan_children(root: &Path) -> Vec<VaultEntry> {
        VaultEntry::scan(root).children
    }

    fn check_in_vault(&self, path: &Path) -> std::io::Result<PathBuf> {
        let canonical_root = self.root.canonicalize()?;
        let canonical_path = if path.exists() {
            path.canonicalize()?
        } else {
            let parent = path.parent().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "No parent")
            })?;
            parent.canonicalize()?.join(path.file_name().unwrap())
        };
        if !canonical_path.starts_with(&canonical_root) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Path escapes vault root",
            ));
        }
        Ok(canonical_path)
    }

    pub fn read_file(&self, path: &Path) -> std::io::Result<String> {
        self.check_in_vault(path)?;
        fs::read_to_string(path)
    }

    pub fn write_file(&self, path: &Path, content: &str) -> std::io::Result<()> {
        self.check_in_vault(path)?;
        fs::write(path, content)
    }

    pub fn create_note(&mut self, parent_dir: &Path, name: &str) -> std::io::Result<PathBuf> {
        let mut file_name = name.to_string();
        if !file_name.ends_with(".md") {
            file_name.push_str(".md");
        }
        let path = parent_dir.join(&file_name);
        let title = name.trim_end_matches(".md");
        self.write_file(&path, &format!("# {}\n\n", title))?;
        self.refresh();
        Ok(path)
    }

    pub fn create_folder(&mut self, parent_dir: &Path, name: &str) -> std::io::Result<PathBuf> {
        let path = parent_dir.join(name);
        fs::create_dir(&path)?;
        self.refresh();
        Ok(path)
    }

    pub fn delete_entry(&mut self, path: &Path) -> std::io::Result<()> {
        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
        self.refresh();
        Ok(())
    }

    pub fn rename_entry(&mut self, path: &Path, new_name: &str) -> std::io::Result<PathBuf> {
        let parent = path.parent().unwrap_or(path);
        let new_path = parent.join(new_name);
        fs::rename(path, &new_path)?;
        self.refresh();
        Ok(new_path)
    }
}
