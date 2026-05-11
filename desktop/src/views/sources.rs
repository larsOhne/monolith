use std::sync::mpsc;

use crate::{
    api::{Client, Source},
    app::{palette, View},
};

enum Msg {
    Sources(Vec<Source>),
    Ingested(Source),
    Error(String),
}

#[derive(Default)]
pub struct SourcesView {
    sources: Vec<Source>,
    loaded_for: Option<String>,
    loading: bool,
    error: Option<String>,
    rx: Option<mpsc::Receiver<Msg>>,
    url_input: String,
    url_input_open: bool,
}

impl SourcesView {
    fn fetch(&mut self, client: &Client, slug: &str) {
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

    fn ingest_file(&mut self, client: &Client, slug: &str, path: String) {
        let (tx, rx) = mpsc::channel::<Msg>();
        self.rx = Some(rx);
        self.loading = true;
        let c = client.clone();
        let s = slug.to_string();
        std::thread::spawn(move || {
            match c.ingest_source(&s, &path) {
                Ok(src) => { let _ = tx.send(Msg::Ingested(src)); }
                Err(e) => { let _ = tx.send(Msg::Error(e.to_string())); }
            }
        });
    }

    fn ingest_url(&mut self, client: &Client, slug: &str, url: String) {
        let (tx, rx) = mpsc::channel::<Msg>();
        self.rx = Some(rx);
        self.loading = true;
        let c = client.clone();
        let s = slug.to_string();
        std::thread::spawn(move || {
            match c.ingest_source_url(&s, &url) {
                Ok(src) => { let _ = tx.send(Msg::Ingested(src)); }
                Err(e) => { let _ = tx.send(Msg::Error(e.to_string())); }
            }
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui, client: &Client, slug: &str, _nav: &mut View) {
        // reload when project changes
        if self.loaded_for.as_deref() != Some(slug) {
            self.sources.clear();
            self.fetch(client, slug);
        }

        if let Some(ref rx) = self.rx {
            while let Ok(msg) = rx.try_recv() {
                self.loading = false;
                match msg {
                    Msg::Sources(list) => self.sources = list,
                    Msg::Ingested(src) => self.sources.push(src),
                    Msg::Error(e) => self.error = Some(e),
                }
            }
        }

        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("Sources").color(palette::PURPLE));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(egui::RichText::new("+ Add file").color(palette::GREEN)).clicked() {
                    // native file dialog
                    let path = rfd::FileDialog::new().pick_file();
                    if let Some(p) = path {
                        self.ingest_file(client, slug, p.to_string_lossy().into_owned());
                    }
                }
                if ui.button(egui::RichText::new("+ Add URL").color(palette::ACCENT)).clicked() {
                    self.url_input_open = !self.url_input_open;
                    self.url_input.clear();
                }
                if ui.button(egui::RichText::new("↻ Refresh").color(palette::MUTED)).clicked() {
                    self.fetch(client, slug);
                }
                if self.loading {
                    ui.spinner();
                }
            });
        });
        ui.separator();

        // Inline URL ingestion form
        if self.url_input_open {
            let mut ingest_url: Option<String> = None;
            egui::Frame::new()
                .fill(palette::SURFACE)
                .stroke(egui::Stroke::new(1.0, palette::ACCENT))
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("URL:").color(palette::MUTED).size(13.0));
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.url_input)
                                .hint_text("https://example.com/paper.pdf")
                                .desired_width(ui.available_width() - 120.0)
                                .font(egui::TextStyle::Monospace),
                        );
                        resp.request_focus();
                        let can_submit = !self.url_input.trim().is_empty();
                        if (ui.add_enabled(can_submit, egui::Button::new("Ingest")).clicked()
                            || (can_submit && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                            && !self.loading
                        {
                            ingest_url = Some(self.url_input.trim().to_string());
                        }
                        if ui.button("✕").clicked() {
                            self.url_input_open = false;
                            self.url_input.clear();
                        }
                    });
                });
            if let Some(url) = ingest_url {
                self.url_input_open = false;
                self.url_input.clear();
                self.ingest_url(client, slug, url);
            }
            ui.add_space(8.0);
        }

        if let Some(ref e) = self.error.clone() {
            ui.label(egui::RichText::new(e).color(palette::RED));
            ui.add_space(8.0);
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for src in &self.sources {
                egui::Frame::new()
                    .fill(palette::SURFACE)
                    .inner_margin(egui::Margin::same(10))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&src.path).color(palette::CYAN).strong());
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("sha256: {:.12}…", src.sha256))
                                    .color(palette::MUTED).size(11.0),
                            );
                            ui.label(
                                egui::RichText::new(format!("git: {:.8}", src.git_sha))
                                    .color(palette::MUTED).size(11.0),
                            );
                        });
                        ui.label(
                            egui::RichText::new(format!("ingested {}", &src.ingested_at[..10]))
                                .color(palette::MUTED).size(11.0),
                        );
                    });
                ui.add_space(4.0);
            }
        });
    }
}
