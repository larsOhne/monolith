//! Obsidian-like markdown vault editor.

use std::{
    path::PathBuf,
    sync::mpsc,
};

use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::vault::Vault;

// ─── Obsidian palette ───────────────────────────────────────────────────────

mod palette {
    use egui::Color32;
    pub const CANVAS: Color32 = Color32::from_rgb(0x0D, 0x0D, 0x0D);
    pub const BG: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x2E);
    pub const SIDEBAR_BG: Color32 = Color32::from_rgb(0x16, 0x16, 0x20);
    pub const SURFACE: Color32 = Color32::from_rgb(0x2A, 0x2A, 0x3A);
    pub const BORDER: Color32 = Color32::from_rgb(0x3A, 0x3A, 0x50);
    pub const FG: Color32 = Color32::from_rgb(0xDC, 0xDD, 0xDE);
    pub const MUTED: Color32 = Color32::from_rgb(0x72, 0x72, 0x8A);
    pub const ACCENT: Color32 = Color32::from_rgb(0x71, 0x53, 0xC6);
    pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x8B, 0x6F, 0xDC);
    pub const ACTIVE_FILE: Color32 = Color32::from_rgb(0x30, 0x2B, 0x4A);
    pub const GREEN: Color32 = Color32::from_rgb(0x4C, 0xAF, 0x50);
}

// ─── Dialog state ───────────────────────────────────────────────────────────

#[derive(Default)]
struct NewItemDialog {
    open: bool,
    is_folder: bool,
    parent: Option<PathBuf>,
    name: String,
}

#[derive(Default)]
struct RenameDialog {
    open: bool,
    target: Option<PathBuf>,
    new_name: String,
}

#[derive(Default)]
struct ConfirmDeleteDialog {
    open: bool,
    target: Option<PathBuf>,
}

#[derive(Default)]
struct OpenVaultDialog {
    open: bool,
    path: String,
    error: Option<String>,
}

// ─── App ────────────────────────────────────────────────────────────────────

pub struct MonolithApp {
    vault: Option<Vault>,
    expanded_dirs: std::collections::HashSet<PathBuf>,
    selected_file: Option<PathBuf>,
    editor_content: String,
    is_dirty: bool,
    /// Index of the block (blank-line-separated paragraph) currently being edited.
    /// `None` means all blocks are rendered as markdown.
    active_block: Option<usize>,
    md_cache: CommonMarkCache,
    sidebar_width: f32,
    // File watcher for live reload
    _watcher: Option<RecommendedWatcher>,
    watcher_rx: Option<mpsc::Receiver<PathBuf>>,
    status_message: Option<(String, std::time::Instant)>,
    new_item_dialog: NewItemDialog,
    rename_dialog: RenameDialog,
    delete_dialog: ConfirmDeleteDialog,
    open_vault_dialog: OpenVaultDialog,
    search_query: String,
    search_results: Vec<PathBuf>,
    search_mode: bool,
}

impl MonolithApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_theme(&cc.egui_ctx);
        Self {
            vault: None,
            expanded_dirs: std::collections::HashSet::new(),
            selected_file: None,
            editor_content: String::new(),
            is_dirty: false,
            active_block: None,
            md_cache: CommonMarkCache::default(),
            sidebar_width: 260.0,
            _watcher: None,
            watcher_rx: None,
            status_message: None,
            new_item_dialog: NewItemDialog::default(),
            rename_dialog: RenameDialog::default(),
            delete_dialog: ConfirmDeleteDialog::default(),
            open_vault_dialog: OpenVaultDialog::default(),
            search_query: String::new(),
            search_results: Vec::new(),
            search_mode: false,
        }
    }

    fn open_file(&mut self, path: PathBuf) {
        self.autosave();
        if let Some(ref vault) = self.vault {
            match vault.read_file(&path) {
                Ok(content) => {
                    self.editor_content = content;
                    self.selected_file = Some(path.clone());
                    self.is_dirty = false;
                    self.active_block = None;
                    self.setup_watcher(path);
                }
                Err(e) => self.set_status(format!("Error reading file: {e}")),
            }
        }
    }

    fn setup_watcher(&mut self, path: PathBuf) {
        let (tx, rx) = mpsc::channel::<PathBuf>();
        let watch_path = path.clone();
        let mut w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                use notify::EventKind::*;
                match event.kind {
                    Modify(_) | Create(_) => {
                        let _ = tx.send(watch_path.clone());
                    }
                    _ => {}
                }
            }
        });
        if let Ok(ref mut watcher) = w {
            let _ = watcher.watch(&path, RecursiveMode::NonRecursive);
        }
        self._watcher = w.ok();
        self.watcher_rx = Some(rx);
    }

    /// Poll watcher channel; reload file if it changed externally.
    fn poll_watcher(&mut self, ctx: &egui::Context) {
        if let Some(ref rx) = self.watcher_rx {
            // Drain all pending events (use the last one)
            let mut changed = false;
            while rx.try_recv().is_ok() {
                changed = true;
            }
            if changed && !self.is_dirty {
                if let (Some(path), Some(ref vault)) =
                    (self.selected_file.clone(), self.vault.as_ref())
                {
                    if let Ok(content) = vault.read_file(&path) {
                        self.editor_content = content;
                        self.active_block = None;
                        ctx.request_repaint();
                    }
                }
            }
        }
    }

    fn save_current(&mut self) {
        if let (Some(path), Some(vault)) = (self.selected_file.clone(), self.vault.as_mut()) {
            let content = self.editor_content.clone();
            match vault.write_file(&path, &content) {
                Ok(_) => {
                    self.is_dirty = false;
                    self.set_status("Saved".to_string());
                }
                Err(e) => self.set_status(format!("Save error: {e}")),
            }
        }
    }

    fn autosave(&mut self) {
        if self.is_dirty {
            self.save_current();
        }
    }

    fn set_status(&mut self, msg: String) {
        self.status_message = Some((msg, std::time::Instant::now()));
    }

    fn open_vault_dialog(&mut self) {
        self.autosave();
        // Pre-fill with current vault root or home dir
        let initial = self
            .vault
            .as_ref()
            .map(|v| v.root.display().to_string())
            .or_else(|| std::env::var("HOME").ok())
            .unwrap_or_default();
        self.open_vault_dialog = OpenVaultDialog {
            open: true,
            path: initial,
            error: None,
        };
    }

    fn do_open_vault(&mut self, path: String) {
        let dir = std::path::PathBuf::from(&path);
        if !dir.is_dir() {
            self.open_vault_dialog.error = Some(format!("{path} is not a directory"));
            return;
        }
        let vault = Vault::open(dir.clone());
        self.vault = Some(vault);
        self.expanded_dirs.clear();
        self.selected_file = None;
        self.editor_content = String::new();
        self.is_dirty = false;
        self.open_vault_dialog.open = false;
        self.set_status(format!("Opened vault: {}", dir.display()));
    }

    fn search_vault(&mut self) {
        self.search_results.clear();
        let query = self.search_query.to_lowercase();
        if query.is_empty() {
            return;
        }
        if let Some(ref vault) = self.vault {
            fn walk(
                entry: &crate::vault::VaultEntry,
                query: &str,
                results: &mut Vec<PathBuf>,
            ) {
                if !entry.is_dir {
                    if entry.name.to_lowercase().contains(query) {
                        results.push(entry.path.clone());
                        return;
                    }
                    if let Ok(content) = std::fs::read_to_string(&entry.path) {
                        if content.to_lowercase().contains(query) {
                            results.push(entry.path.clone());
                        }
                    }
                }
                for child in &entry.children {
                    walk(child, query, results);
                }
            }
            let tree_snapshot: Vec<_> = vault.tree.iter().cloned().collect();
            for entry in &tree_snapshot {
                walk(entry, &query, &mut self.search_results);
            }
        }
    }

    // ── Rendering ────────────────────────────────────────────────────────────

    fn render_title_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("title_bar")
            .exact_height(40.0)
            .frame(
                egui::Frame::none()
                    .fill(palette::SIDEBAR_BG)
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let vault_name = self
                        .vault
                        .as_ref()
                        .map(|v| {
                            v.root
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_default()
                        })
                        .unwrap_or_else(|| "No vault open".to_string());
                    ui.label(
                        egui::RichText::new(format!("⬡  {vault_name}"))
                            .color(palette::FG)
                            .size(14.0)
                            .strong(),
                    );

                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.separator();

                            if self.is_dirty
                                && ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("● Save")
                                                .color(palette::ACCENT)
                                                .size(13.0),
                                        )
                                        .frame(false),
                                    )
                                    .on_hover_text("Ctrl+S")
                                    .clicked()
                            {
                                self.save_current();
                            }

                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Open Vault")
                                            .color(palette::MUTED)
                                            .size(13.0),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                            {
                                self.open_vault_dialog();
                            }
                        },
                    );
                });
            });
    }

    fn render_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(22.0)
            .frame(
                egui::Frame::none()
                    .fill(palette::SIDEBAR_BG)
                    .inner_margin(egui::Margin::symmetric(12, 2)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if let Some(ref path) = self.selected_file.clone() {
                        ui.label(
                            egui::RichText::new(path.display().to_string())
                                .color(palette::MUTED)
                                .size(11.0),
                        );
                    }

                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if let Some((ref msg, ts)) = self.status_message.clone() {
                                if ts.elapsed().as_secs() < 3 {
                                    ui.label(
                                        egui::RichText::new(msg)
                                            .color(palette::GREEN)
                                            .size(11.0),
                                    );
                                } else {
                                    self.status_message = None;
                                }
                            }
                            if self.is_dirty {
                                ui.label(
                                    egui::RichText::new("●")
                                        .color(palette::ACCENT)
                                        .size(11.0),
                                );
                            }
                        },
                    );
                });
            });
    }

    fn render_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(self.sidebar_width)
            .min_width(160.0)
            .max_width(400.0)
            .frame(egui::Frame::none().fill(palette::SIDEBAR_BG))
            .show(ctx, |ui| {
                egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(8, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let search_color = if self.search_mode {
                                palette::ACCENT
                            } else {
                                palette::MUTED
                            };
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("🔍").color(search_color),
                                    )
                                    .frame(false),
                                )
                                .on_hover_text("Search (Ctrl+F)")
                                .clicked()
                            {
                                self.search_mode = !self.search_mode;
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if self.vault.is_some() {
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    egui::RichText::new("📁+")
                                                        .color(palette::MUTED),
                                                )
                                                .frame(false),
                                            )
                                            .on_hover_text("New folder")
                                            .clicked()
                                        {
                                            let root =
                                                self.vault.as_ref().unwrap().root.clone();
                                            self.new_item_dialog = NewItemDialog {
                                                open: true,
                                                is_folder: true,
                                                parent: Some(root),
                                                name: String::new(),
                                            };
                                        }
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    egui::RichText::new("📄+")
                                                        .color(palette::MUTED),
                                                )
                                                .frame(false),
                                            )
                                            .on_hover_text("New note")
                                            .clicked()
                                        {
                                            let root =
                                                self.vault.as_ref().unwrap().root.clone();
                                            self.new_item_dialog = NewItemDialog {
                                                open: true,
                                                is_folder: false,
                                                parent: Some(root),
                                                name: String::new(),
                                            };
                                        }
                                    }
                                },
                            );
                        });

                        if self.search_mode {
                            ui.add_space(4.0);
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.search_query)
                                    .hint_text("Search notes…")
                                    .desired_width(f32::INFINITY)
                                    .font(egui::TextStyle::Small),
                            );
                            if resp.changed() {
                                self.search_vault();
                            }
                        }
                    });

                egui::ScrollArea::vertical()
                    .id_salt("sidebar_scroll")
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());

                        if self.search_mode && !self.search_query.is_empty() {
                            let results = self.search_results.clone();
                            for path in results {
                                let name = path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                let is_sel = self.selected_file.as_deref() == Some(&path);
                                let label = format!(
                                    "  {}",
                                    name.trim_end_matches(".md").trim_end_matches(".txt")
                                );
                                let color =
                                    if is_sel { palette::ACCENT_HOVER } else { palette::FG };
                                let bg = if is_sel {
                                    palette::ACTIVE_FILE
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new(&label).color(color).size(13.0),
                                        )
                                        .frame(is_sel)
                                        .fill(bg)
                                        .min_size(egui::vec2(ui.available_width(), 22.0)),
                                    )
                                    .clicked()
                                {
                                    self.open_file(path);
                                }
                            }
                        } else if let Some(ref vault) = self.vault {
                            let entries: Vec<_> = vault.tree.iter().cloned().collect();
                            let mut open_file: Option<PathBuf> = None;
                            let mut toggle_dir: Option<PathBuf> = None;
                            let mut new_note_in: Option<PathBuf> = None;
                            let mut new_folder_in: Option<PathBuf> = None;
                            let mut rename_target: Option<PathBuf> = None;
                            let mut delete_target: Option<PathBuf> = None;

                            Self::render_tree(
                                ui,
                                &entries,
                                0,
                                &self.selected_file,
                                &self.expanded_dirs,
                                &mut open_file,
                                &mut toggle_dir,
                                &mut new_note_in,
                                &mut new_folder_in,
                                &mut rename_target,
                                &mut delete_target,
                            );

                            if let Some(path) = open_file {
                                self.open_file(path);
                            }
                            if let Some(dir) = toggle_dir {
                                if self.expanded_dirs.contains(&dir) {
                                    self.expanded_dirs.remove(&dir);
                                } else {
                                    self.expanded_dirs.insert(dir);
                                }
                            }
                            if let Some(dir) = new_note_in {
                                self.new_item_dialog = NewItemDialog {
                                    open: true,
                                    is_folder: false,
                                    parent: Some(dir),
                                    name: String::new(),
                                };
                            }
                            if let Some(dir) = new_folder_in {
                                self.new_item_dialog = NewItemDialog {
                                    open: true,
                                    is_folder: true,
                                    parent: Some(dir),
                                    name: String::new(),
                                };
                            }
                            if let Some(target) = rename_target {
                                let current_name = target
                                    .file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                self.rename_dialog = RenameDialog {
                                    open: true,
                                    target: Some(target),
                                    new_name: current_name,
                                };
                            }
                            if let Some(target) = delete_target {
                                self.delete_dialog =
                                    ConfirmDeleteDialog { open: true, target: Some(target) };
                            }
                        } else {
                            ui.add_space(40.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    egui::RichText::new("No vault open")
                                        .color(palette::MUTED)
                                        .size(13.0),
                                );
                                ui.add_space(12.0);
                                if ui.button("Open Folder…").clicked() {
                                    self.open_vault_dialog();
                                }
                            });
                        }
                    });
            });
    }

    #[allow(clippy::too_many_arguments)]
    fn render_tree(
        ui: &mut egui::Ui,
        entries: &[crate::vault::VaultEntry],
        depth: usize,
        selected: &Option<PathBuf>,
        expanded: &std::collections::HashSet<PathBuf>,
        open_file: &mut Option<PathBuf>,
        toggle_dir: &mut Option<PathBuf>,
        new_note_in: &mut Option<PathBuf>,
        new_folder_in: &mut Option<PathBuf>,
        rename_target: &mut Option<PathBuf>,
        delete_target: &mut Option<PathBuf>,
    ) {
        for entry in entries {
            let indent = depth as f32 * 16.0;
            let _ = indent; // used logically via string padding

            if entry.is_dir {
                let is_open = expanded.contains(&entry.path);
                let icon = if is_open { "▾" } else { "▸" };
                let label =
                    format!("{:pad$}{icon} {}", "", entry.name, pad = depth * 2);

                let resp = ui.add(
                    egui::Button::new(
                        egui::RichText::new(&label).color(palette::FG).size(13.0),
                    )
                    .frame(false)
                    .min_size(egui::vec2(ui.available_width(), 22.0)),
                );

                resp.context_menu(|ui| {
                    if ui.button("New Note Here").clicked() {
                        *new_note_in = Some(entry.path.clone());
                        ui.close_menu();
                    }
                    if ui.button("New Folder Here").clicked() {
                        *new_folder_in = Some(entry.path.clone());
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Rename").clicked() {
                        *rename_target = Some(entry.path.clone());
                        ui.close_menu();
                    }
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new("Delete").color(egui::Color32::RED),
                        ))
                        .clicked()
                    {
                        *delete_target = Some(entry.path.clone());
                        ui.close_menu();
                    }
                });

                if resp.clicked() {
                    *toggle_dir = Some(entry.path.clone());
                }

                if is_open {
                    Self::render_tree(
                        ui,
                        &entry.children,
                        depth + 1,
                        selected,
                        expanded,
                        open_file,
                        toggle_dir,
                        new_note_in,
                        new_folder_in,
                        rename_target,
                        delete_target,
                    );
                }
            } else if entry.name.ends_with(".md") || entry.name.ends_with(".txt") {
                let is_selected = selected.as_deref() == Some(entry.path.as_path());
                let display =
                    entry.name.trim_end_matches(".md").trim_end_matches(".txt");
                let label =
                    format!("{:pad$}  {display}", "", pad = depth * 2);
                let color =
                    if is_selected { palette::ACCENT_HOVER } else { palette::FG };
                let bg = if is_selected {
                    palette::ACTIVE_FILE
                } else {
                    egui::Color32::TRANSPARENT
                };

                let resp = ui.add(
                    egui::Button::new(
                        egui::RichText::new(&label).color(color).size(13.0),
                    )
                    .frame(is_selected)
                    .fill(bg)
                    .min_size(egui::vec2(ui.available_width(), 22.0)),
                );

                resp.context_menu(|ui| {
                    if ui.button("Rename").clicked() {
                        *rename_target = Some(entry.path.clone());
                        ui.close_menu();
                    }
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new("Delete").color(egui::Color32::RED),
                        ))
                        .clicked()
                    {
                        *delete_target = Some(entry.path.clone());
                        ui.close_menu();
                    }
                });

                if resp.clicked() {
                    *open_file = Some(entry.path.clone());
                }
            }
        }
    }

    fn render_editor(&mut self, ui: &mut egui::Ui) {
        if self.selected_file.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Select or create a note to get started")
                        .color(palette::MUTED)
                        .size(16.0),
                );
            });
            return;
        }

        // Split content into blank-line-separated blocks.
        // We edit blocks[active_block] in-place via TextEdit; others render as markdown.
        let mut blocks: Vec<String> = self.editor_content
            .split("\n\n")
            .map(|s| s.to_string())
            .collect();
        if blocks.is_empty() {
            blocks.push(String::new());
        }

        // Clamp active_block index in case blocks were removed externally.
        if let Some(idx) = self.active_block {
            if idx >= blocks.len() {
                self.active_block = Some(blocks.len() - 1);
            }
        }

        let mut clicked_block: Option<usize> = None;
        let mut should_deactivate = false;
        let mut content_changed = false;

        egui::ScrollArea::vertical()
            .id_salt("live_scroll")
            .show(ui, |ui| {
                egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(48, 32))
                    .show(ui, |ui| {
                        ui.set_max_width(760.0);

                        let n = blocks.len();
                        for i in 0..n {
                            let is_active = self.active_block == Some(i);

                            if is_active {
                                let te_id = egui::Id::new(("live_te", i));
                                let resp = ui.add(
                                    egui::TextEdit::multiline(&mut blocks[i])
                                        .id(te_id)
                                        .desired_width(f32::INFINITY)
                                        .font(egui::FontId::monospace(14.0))
                                        .frame(egui::Frame::NONE),
                                );
                                if resp.changed() {
                                    content_changed = true;
                                }
                                // Grab focus the moment this block activates.
                                if !resp.has_focus() {
                                    resp.request_focus();
                                }
                                // Escape → back to preview
                                if ui.input(|inp| inp.key_pressed(egui::Key::Escape)) {
                                    should_deactivate = true;
                                }
                                // Clicking elsewhere deactivates too
                                if resp.lost_focus() && clicked_block.is_none() {
                                    should_deactivate = true;
                                }
                            } else {
                                let top_y = ui.cursor().min.y;
                                let block_content = &blocks[i];

                                ui.push_id(("bv", i), |ui| {
                                    if block_content.trim().is_empty() {
                                        // Invisible spacer so empty paragraphs are clickable
                                        ui.add_space(20.0);
                                    } else {
                                        CommonMarkViewer::new()
                                            .show(ui, &mut self.md_cache, block_content.as_str());
                                    }
                                });

                                let bottom_y = ui.cursor().min.y;
                                let block_rect = egui::Rect::from_min_max(
                                    egui::pos2(ui.max_rect().min.x, top_y),
                                    egui::pos2(
                                        ui.max_rect().max.x,
                                        bottom_y.max(top_y + 20.0),
                                    ),
                                );
                                let cr =
                                    ui.allocate_rect(block_rect, egui::Sense::click());
                                if cr.clicked() {
                                    clicked_block = Some(i);
                                }
                                if cr.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                                }
                            }

                            ui.add_space(4.0);
                        }

                        // Clicking empty space below content activates last block.
                        if self.active_block.is_none() {
                            let remaining = ui.available_height().max(40.0);
                            let footer =
                                ui.allocate_response(
                                    egui::Vec2::new(ui.available_width(), remaining),
                                    egui::Sense::click(),
                                );
                            if footer.clicked() {
                                clicked_block = Some(n - 1);
                            }
                        }
                    });
            });

        // Sync edited block back into editor_content.
        if content_changed {
            self.editor_content = blocks.join("\n\n");
            self.is_dirty = true;
        }

        // State transitions (do after rendering to avoid borrow conflicts).
        if should_deactivate {
            self.active_block = None;
        } else if let Some(idx) = clicked_block {
            self.active_block = Some(idx);
        }
    }

    // ── Dialogs ──────────────────────────────────────────────────────────────

    fn show_new_item_dialog(&mut self, ctx: &egui::Context) {
        if !self.new_item_dialog.open {
            return;
        }
        let title = if self.new_item_dialog.is_folder {
            "New Folder"
        } else {
            "New Note"
        };
        let mut open = true;
        let mut do_create = false;

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.new_item_dialog.name)
                            .desired_width(260.0)
                            .hint_text(if self.new_item_dialog.is_folder {
                                "folder-name"
                            } else {
                                "Note title"
                            }),
                    );
                    resp.request_focus();
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let can = !self.new_item_dialog.name.trim().is_empty();
                    if ui.add_enabled(can, egui::Button::new("Create")).clicked()
                        || (can && ctx.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        do_create = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.new_item_dialog.open = false;
                        self.new_item_dialog.name.clear();
                    }
                });
            });

        if !open {
            self.new_item_dialog.open = false;
            self.new_item_dialog.name.clear();
        }

        if do_create {
            let name = self.new_item_dialog.name.trim().to_string();
            let is_folder = self.new_item_dialog.is_folder;
            let parent = self.new_item_dialog.parent.clone();
            self.new_item_dialog.open = false;
            self.new_item_dialog.name.clear();

            if let (Some(parent), Some(vault)) = (parent, self.vault.as_mut()) {
                if is_folder {
                    if let Err(e) = vault.create_folder(&parent, &name) {
                        self.set_status(format!("Error: {e}"));
                    }
                } else {
                    match vault.create_note(&parent, &name) {
                        Ok(path) => {
                            // Open the new note immediately
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                self.editor_content = content;
                                self.selected_file = Some(path);
                                self.is_dirty = false;
                            }
                        }
                        Err(e) => self.set_status(format!("Error: {e}")),
                    }
                }
            }
        }
    }

    fn show_rename_dialog(&mut self, ctx: &egui::Context) {
        if !self.rename_dialog.open {
            return;
        }
        let mut open = true;
        let mut do_rename = false;

        egui::Window::new("Rename")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("New name:");
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.rename_dialog.new_name)
                            .desired_width(260.0),
                    );
                    resp.request_focus();
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let can = !self.rename_dialog.new_name.trim().is_empty();
                    if ui.add_enabled(can, egui::Button::new("Rename")).clicked()
                        || (can && ctx.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        do_rename = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.rename_dialog.open = false;
                    }
                });
            });

        if !open {
            self.rename_dialog.open = false;
        }

        if do_rename {
            let new_name = self.rename_dialog.new_name.trim().to_string();
            let target = self.rename_dialog.target.clone();
            self.rename_dialog.open = false;

            if let (Some(target), Some(vault)) = (target, self.vault.as_mut()) {
                match vault.rename_entry(&target, &new_name) {
                    Ok(new_path) => {
                        if self.selected_file.as_deref() == Some(&target) {
                            self.selected_file = Some(new_path);
                        }
                    }
                    Err(e) => self.set_status(format!("Rename error: {e}")),
                }
            }
        }
    }

    fn show_delete_dialog(&mut self, ctx: &egui::Context) {
        if !self.delete_dialog.open {
            return;
        }
        let target_name = self
            .delete_dialog
            .target
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut open = true;
        let mut do_delete = false;

        egui::Window::new("Confirm Delete")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(format!("Delete \u{00ab}{target_name}\u{00bb}?"));
                ui.label(
                    egui::RichText::new("This cannot be undone.")
                        .color(egui::Color32::RED)
                        .size(12.0),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new("Delete").color(egui::Color32::RED),
                        ))
                        .clicked()
                    {
                        do_delete = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.delete_dialog.open = false;
                    }
                });
            });

        if !open {
            self.delete_dialog.open = false;
        }

        if do_delete {
            let target = self.delete_dialog.target.clone();
            self.delete_dialog.open = false;
            if let (Some(target), Some(vault)) = (target, self.vault.as_mut()) {
                match vault.delete_entry(&target) {
                    Ok(_) => {
                        if self.selected_file.as_deref() == Some(&target) {
                            self.selected_file = None;
                            self.editor_content.clear();
                            self.is_dirty = false;
                        }
                    }
                    Err(e) => self.set_status(format!("Delete error: {e}")),
                }
            }
        }
    }

    fn show_open_vault_dialog(&mut self, ctx: &egui::Context) {
        if !self.open_vault_dialog.open {
            return;
        }
        let mut open = true;
        let mut do_open: Option<String> = None;

        egui::Window::new("Open Vault")
            .collapsible(false)
            .resizable(false)
            .min_width(420.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Folder path:");
                ui.add_space(4.0);
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.open_vault_dialog.path)
                        .desired_width(f32::INFINITY)
                        .hint_text("/home/user/my-vault"),
                );
                resp.request_focus();

                if let Some(ref err) = self.open_vault_dialog.error.clone() {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(err).color(egui::Color32::RED).size(12.0));
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let can = !self.open_vault_dialog.path.trim().is_empty();
                    if ui.add_enabled(can, egui::Button::new("Open")).clicked()
                        || (can && ctx.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        do_open = Some(self.open_vault_dialog.path.trim().to_string());
                    }
                    if ui.button("Cancel").clicked() {
                        self.open_vault_dialog.open = false;
                    }
                });
            });

        if !open {
            self.open_vault_dialog.open = false;
        }

        if let Some(path) = do_open {
            self.do_open_vault(path);
        }
    }
}

// ─── eframe::App impl ───────────────────────────────────────────────────────

impl eframe::App for MonolithApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Poll file watcher for external changes
        self.poll_watcher(&ctx);
        // Request continuous repaints while file is open (watcher debounce)
        if self.selected_file.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }

        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
            self.save_current();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::F)) {
            self.search_mode = !self.search_mode;
        }

        self.render_title_bar(&ctx);
        self.render_status_bar(&ctx);
        self.render_sidebar(&ctx);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(palette::BG)
                    .inner_margin(egui::Margin::same(0)),
            )
            .show(&ctx, |ui| {
                self.render_editor(ui);
            });

        self.show_new_item_dialog(&ctx);
        self.show_rename_dialog(&ctx);
        self.show_delete_dialog(&ctx);
        self.show_open_vault_dialog(&ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.autosave();
    }
}

// ─── Theme ──────────────────────────────────────────────────────────────────

fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = palette::BG;
    visuals.window_fill = palette::SURFACE;
    visuals.faint_bg_color = palette::CANVAS;
    visuals.extreme_bg_color = palette::CANVAS;
    visuals.code_bg_color = palette::CANVAS;
    visuals.override_text_color = Some(palette::FG);
    visuals.selection.bg_fill = palette::ACCENT.gamma_multiply(0.4);
    visuals.selection.stroke.color = palette::ACCENT;
    visuals.widgets.noninteractive.bg_fill = palette::SURFACE;
    visuals.widgets.noninteractive.fg_stroke.color = palette::MUTED;
    visuals.widgets.noninteractive.bg_stroke.color = palette::BORDER;
    visuals.widgets.inactive.bg_fill = palette::SURFACE;
    visuals.widgets.inactive.fg_stroke.color = palette::FG;
    visuals.widgets.hovered.bg_fill = palette::SURFACE;
    visuals.widgets.hovered.fg_stroke.color = palette::FG;
    visuals.widgets.active.bg_fill = palette::ACCENT;
    visuals.widgets.active.fg_stroke.color = palette::FG;
    visuals.window_corner_radius = egui::CornerRadius::same(8);
    visuals.window_shadow = egui::Shadow {
        offset: [4, 8],
        blur: 16,
        spread: 0,
        color: egui::Color32::from_black_alpha(120),
    };
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(4.0, 2.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    ctx.set_style(style);
}
