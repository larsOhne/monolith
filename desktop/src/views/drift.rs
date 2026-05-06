use std::sync::{mpsc, Arc, Mutex};

use crate::api::{Client, DriftEntry, DriftReport};
use crate::app::palette;

enum Msg {
    Report(DriftReport),
    Error(String),
}

#[derive(Default)]
pub struct DriftView {
    entries: Vec<DriftEntry>,
    loaded_for: Option<String>,
    loading: bool,
    error: Option<String>,
    rx: Option<mpsc::Receiver<Msg>>,
}

impl DriftView {
    fn fetch(&mut self, client: &Client, slug: &str) {
        let (tx, rx) = mpsc::channel::<Msg>();
        self.rx = Some(rx);
        self.loading = true;
        self.error = None;
        self.loaded_for = Some(slug.to_string());
        let c = client.clone();
        let s = slug.to_string();
        std::thread::spawn(move || {
            match c.check_drift(&s) {
                Ok(r) => { let _ = tx.send(Msg::Report(r)); }
                Err(e) => { let _ = tx.send(Msg::Error(e.to_string())); }
            }
        });
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        client: &Client,
        slug: &str,
        drift_count: &mut Arc<Mutex<usize>>,
    ) {
        if self.loaded_for.as_deref() != Some(slug) {
            self.entries.clear();
            self.fetch(client, slug);
        }

        if let Some(ref rx) = self.rx {
            while let Ok(msg) = rx.try_recv() {
                self.loading = false;
                match msg {
                    Msg::Report(r) => {
                        let broken = r.entries.iter().filter(|e| e.status != "valid").count();
                        *drift_count.lock().unwrap() = broken;
                        self.entries = r.entries;
                    }
                    Msg::Error(e) => self.error = Some(e),
                }
            }
        }

        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("Drift Check").color(palette::PURPLE));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(egui::RichText::new("↻ Re-check").color(palette::CYAN)).clicked() {
                    self.loaded_for = None; // force reload
                }
                if self.loading {
                    ui.spinner();
                }
            });
        });
        ui.separator();

        if let Some(ref e) = self.error.clone() {
            ui.label(egui::RichText::new(e).color(palette::RED));
            return;
        }

        if self.entries.is_empty() && !self.loading {
            ui.label(egui::RichText::new("✓ All evidence is valid.").color(palette::GREEN));
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for entry in &self.entries {
                let (color, icon) = match entry.status.as_str() {
                    "valid" => (palette::GREEN, "✓"),
                    "drifted" => (palette::ORANGE, "⚠"),
                    "broken" => (palette::RED, "✗"),
                    _ => (palette::MUTED, "?"),
                };

                egui::Frame::new()
                    .fill(palette::SURFACE)
                    .inner_margin(egui::Margin::same(10))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(icon).color(color).size(16.0));
                            ui.label(
                                egui::RichText::new(&entry.status)
                                    .color(color)
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!("evidence {:.8}…", entry.evidence_id))
                                    .color(palette::MUTED)
                                    .size(11.0),
                            );
                        });
                        if let Some(ref diff) = entry.diff {
                            ui.add_space(4.0);
                            egui::ScrollArea::horizontal().max_height(120.0).show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(diff)
                                        .color(palette::YELLOW)
                                        .size(11.0)
                                        .font(egui::FontId::monospace(11.0)),
                                );
                            });
                        }
                    });
                ui.add_space(4.0);
            }
        });
    }
}
