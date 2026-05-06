use std::sync::mpsc;

use crate::{
    api::{Client, Evidence, Source},
    app::{palette, View},
};

enum Msg {
    Sources(Vec<Source>),
    Content(String),
    Pinned(Evidence),
    Error(String),
}

#[derive(Default)]
pub struct EvidenceView {
    sources: Vec<Source>,
    loaded_for: Option<String>,
    loading: bool,
    error: Option<String>,
    rx: Option<mpsc::Receiver<Msg>>,

    // pin flow
    selected_source: Option<Source>,
    source_content: Option<String>,
    content_loading: bool,
    selection_buf: String,
    pinned_flash: Option<String>, // brief "pinned!" feedback
}

impl EvidenceView {
    fn fetch_sources(&mut self, client: &Client, slug: &str) {
        let (tx, rx) = mpsc::channel::<Msg>();
        self.rx = Some(rx);
        self.loading = true;
        self.error = None;
        self.loaded_for = Some(slug.to_string());
        let c = client.clone();
        let s = slug.to_string();
        std::thread::spawn(move || {
            match c.list_sources(&s) {
                Ok(list) => { let _ = tx.send(Msg::Sources(list)); }
                Err(e) => { let _ = tx.send(Msg::Error(e.to_string())); }
            }
        });
    }

    fn fetch_content(&mut self, client: &Client, source_id: &str) {
        let (tx, rx) = mpsc::channel::<Msg>();
        self.rx = Some(rx);
        self.content_loading = true;
        let c = client.clone();
        let id = source_id.to_string();
        std::thread::spawn(move || {
            match c.source_content(&id) {
                Ok(text) => { let _ = tx.send(Msg::Content(text)); }
                Err(e) => { let _ = tx.send(Msg::Error(e.to_string())); }
            }
        });
    }

    fn pin(&mut self, client: &Client) {
        if let Some(ref src) = self.selected_source.clone() {
            if self.selection_buf.is_empty() {
                return;
            }
            let (tx, rx) = mpsc::channel::<Msg>();
            self.rx = Some(rx);
            let c = client.clone();
            let sid = src.id.clone();
            let text = self.selection_buf.clone();
            std::thread::spawn(move || {
                match c.pin_evidence(&sid, &text) {
                    Ok(ev) => { let _ = tx.send(Msg::Pinned(ev)); }
                    Err(e) => { let _ = tx.send(Msg::Error(e.to_string())); }
                }
            });
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, client: &Client, slug: &str, _nav: &mut View) {
        if self.loaded_for.as_deref() != Some(slug) {
            self.sources.clear();
            self.selected_source = None;
            self.source_content = None;
            self.fetch_sources(client, slug);
        }

        if let Some(ref rx) = self.rx {
            while let Ok(msg) = rx.try_recv() {
                self.loading = false;
                self.content_loading = false;
                match msg {
                    Msg::Sources(list) => self.sources = list,
                    Msg::Content(text) => self.source_content = Some(text),
                    Msg::Pinned(ev) => {
                        self.pinned_flash = Some(format!("Pinned evidence {:.8}…", ev.id));
                        self.selection_buf.clear();
                    }
                    Msg::Error(e) => self.error = Some(e),
                }
            }
        }

        ui.heading(egui::RichText::new("Pin Evidence").color(palette::PURPLE));
        ui.separator();

        if let Some(ref e) = self.error.clone() {
            ui.label(egui::RichText::new(e).color(palette::RED));
        }
        if let Some(ref flash) = self.pinned_flash.clone() {
            ui.label(egui::RichText::new(flash).color(palette::GREEN));
        }

        // Left: source picker | Right: content + pin
        ui.columns(2, |cols| {
            // ── Source list ────────────────────────────────────────────────
            let left = &mut cols[0];
            left.label(egui::RichText::new("Select a source").color(palette::MUTED));
            if self.loading {
                left.spinner();
            }
            egui::ScrollArea::vertical().id_salt("src_list").show(left, |ui| {
                let sources_snapshot = self.sources.clone();
                for src in &sources_snapshot {
                    let selected = self
                        .selected_source
                        .as_ref()
                        .map(|s| s.id == src.id)
                        .unwrap_or(false);
                    let text = if selected {
                        egui::RichText::new(&src.path).color(palette::PURPLE).strong()
                    } else {
                        egui::RichText::new(&src.path).color(palette::FG)
                    };
                    if ui.selectable_label(selected, text).clicked() && !selected {
                        self.selected_source = Some(src.clone());
                        self.source_content = None;
                        self.fetch_content(&client.clone(), &src.id);
                    }
                }
            });

            // ── Content + pin ──────────────────────────────────────────────
            let right = &mut cols[1];
            if let Some(ref content) = self.source_content.clone() {
                right.label(egui::RichText::new("Source content (copy text to pin it below)").color(palette::MUTED).size(11.0));
                egui::ScrollArea::vertical()
                    .id_salt("src_content")
                    .max_height(300.0)
                    .show(right, |ui| {
                        egui::Frame::new()
                            .fill(palette::CANVAS)
                            .inner_margin(egui::Margin::same(8))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(content)
                                        .color(palette::FG)
                                        .size(12.0)
                                        .font(egui::FontId::monospace(12.0)),
                                );
                            });
                    });

                right.add_space(8.0);
                right.label(egui::RichText::new("Verbatim text to pin").color(palette::CYAN));
                right.add(
                    egui::TextEdit::multiline(&mut self.selection_buf)
                        .desired_rows(4)
                        .hint_text("Paste exact passage from the source…"),
                );
                right.add_space(4.0);
                if right
                    .add_enabled(
                        !self.selection_buf.is_empty(),
                        egui::Button::new(egui::RichText::new("Pin evidence").color(palette::GREEN)),
                    )
                    .clicked()
                {
                    self.pin(client);
                }
            } else if self.content_loading {
                right.spinner();
            } else {
                right.label(egui::RichText::new("← Select a source to view its content").color(palette::MUTED));
            }
        });
    }
}
