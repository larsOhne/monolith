use std::sync::mpsc;

use crate::{
    api::{Client, Statement},
    app::{palette, View},
};

enum Msg {
    Statements(Vec<Statement>),
    Created(Statement),
    Error(String),
}

#[derive(Default)]
pub struct StatementsView {
    statements: Vec<Statement>,
    loaded_for: Option<String>,
    loading: bool,
    error: Option<String>,
    rx: Option<mpsc::Receiver<Msg>>,
    // create form
    content_buf: String,
    evidence_ids_buf: String, // comma-separated
    show_create: bool,
}

impl StatementsView {
    fn fetch(&mut self, client: &Client, slug: &str) {
        let (tx, rx) = mpsc::channel::<Msg>();
        self.rx = Some(rx);
        self.loading = true;
        self.error = None;
        self.loaded_for = Some(slug.to_string());
        let c = client.clone();
        let s = slug.to_string();
        std::thread::spawn(move || {
            match c.list_statements(&s) {
                Ok(list) => { let _ = tx.send(Msg::Statements(list)); }
                Err(e) => { let _ = tx.send(Msg::Error(e.to_string())); }
            }
        });
    }

    fn create(&mut self, client: &Client, slug: &str) {
        let (tx, rx) = mpsc::channel::<Msg>();
        self.rx = Some(rx);
        self.loading = true;
        let c = client.clone();
        let s = slug.to_string();
        let content = self.content_buf.clone();
        let ids: Vec<String> = self
            .evidence_ids_buf
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        std::thread::spawn(move || {
            match c.create_statement(&s, &content, &ids) {
                Ok(stmt) => { let _ = tx.send(Msg::Created(stmt)); }
                Err(e) => { let _ = tx.send(Msg::Error(e.to_string())); }
            }
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui, client: &Client, slug: &str, _nav: &mut View) {
        if self.loaded_for.as_deref() != Some(slug) {
            self.statements.clear();
            self.fetch(client, slug);
        }

        if let Some(ref rx) = self.rx {
            while let Ok(msg) = rx.try_recv() {
                self.loading = false;
                match msg {
                    Msg::Statements(list) => self.statements = list,
                    Msg::Created(stmt) => {
                        self.statements.push(stmt);
                        self.show_create = false;
                        self.content_buf.clear();
                        self.evidence_ids_buf.clear();
                    }
                    Msg::Error(e) => self.error = Some(e),
                }
            }
        }

        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("Statements").color(palette::PURPLE));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(egui::RichText::new("+ New").color(palette::GREEN)).clicked() {
                    self.show_create = !self.show_create;
                }
                if self.loading {
                    ui.spinner();
                }
            });
        });
        ui.separator();

        if let Some(ref e) = self.error.clone() {
            ui.label(egui::RichText::new(e).color(palette::RED));
        }

        if self.show_create {
            egui::Frame::new()
                .fill(palette::SURFACE)
                .inner_margin(egui::Margin::same(12))
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("New statement").color(palette::CYAN).strong());
                    ui.add_space(4.0);
                    ui.label("Content");
                    ui.add(egui::TextEdit::multiline(&mut self.content_buf).desired_rows(3));
                    ui.add_space(4.0);
                    ui.label("Evidence IDs (comma-separated)");
                    ui.text_edit_singleline(&mut self.evidence_ids_buf);
                    ui.add_space(4.0);
                    if ui.button(egui::RichText::new("Create").color(palette::GREEN)).clicked()
                        && !self.content_buf.is_empty()
                    {
                        self.create(client, slug);
                    }
                });
            ui.add_space(8.0);
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for stmt in &self.statements {
                egui::Frame::new()
                    .fill(palette::SURFACE)
                    .inner_margin(egui::Margin::same(10))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&stmt.content).color(palette::FG));
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new("Evidence:").color(palette::MUTED).size(11.0));
                            for id in &stmt.evidence_ids {
                                ui.label(
                                    egui::RichText::new(format!("{:.8}…", id))
                                        .color(palette::CYAN)
                                        .background_color(palette::CANVAS)
                                        .size(11.0),
                                );
                            }
                        });
                        ui.label(
                            egui::RichText::new(format!("created {}", &stmt.created_at[..10]))
                                .color(palette::MUTED).size(10.0),
                        );
                    });
                ui.add_space(4.0);
            }
        });
    }
}
