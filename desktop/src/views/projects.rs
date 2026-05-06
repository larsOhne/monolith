use std::sync::{mpsc, Arc, Mutex};

use crate::{
    api::{Client, Project},
    app::{palette, View},
};

enum Msg {
    Projects(Vec<Project>),
    Created(Project),
    Error(String),
}

#[derive(Default)]
pub struct ProjectsView {
    projects: Vec<Project>,
    loading: bool,
    error: Option<String>,
    // create form
    name_buf: String,
    slug_buf: String,
    desc_buf: String,
    show_create: bool,
    tx: Option<mpsc::Sender<()>>,
    rx: Option<mpsc::Receiver<Msg>>,
}

impl ProjectsView {
    fn fetch(&mut self, client: &Client) {
        let (tx_done, _rx_done) = mpsc::channel::<()>();
        let (tx, rx) = mpsc::channel::<Msg>();
        self.rx = Some(rx);
        self.tx = Some(tx_done);
        self.loading = true;
        self.error = None;
        let c = client.clone();
        let tx2 = tx.clone();
        std::thread::spawn(move || {
            match c.list_projects() {
                Ok(list) => { let _ = tx2.send(Msg::Projects(list)); }
                Err(e) => { let _ = tx2.send(Msg::Error(e.to_string())); }
            }
        });
        // store sender so rx outlives thread
        let _ = tx;
    }

    fn create(&mut self, client: &Client) {
        let (tx, rx) = mpsc::channel::<Msg>();
        self.rx = Some(rx);
        self.loading = true;
        self.error = None;
        let c = client.clone();
        let name = self.name_buf.clone();
        let slug = self.slug_buf.clone();
        let desc = self.desc_buf.clone();
        std::thread::spawn(move || {
            match c.create_project(&name, &slug, &desc) {
                Ok(p) => { let _ = tx.send(Msg::Created(p)); }
                Err(e) => { let _ = tx.send(Msg::Error(e.to_string())); }
            }
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui, client: &Client, nav: &mut View) {
        // drain channel
        if let Some(ref rx) = self.rx {
            while let Ok(msg) = rx.try_recv() {
                self.loading = false;
                match msg {
                    Msg::Projects(list) => self.projects = list,
                    Msg::Created(p) => {
                        self.projects.push(p);
                        self.show_create = false;
                        self.name_buf.clear();
                        self.slug_buf.clear();
                        self.desc_buf.clear();
                    }
                    Msg::Error(e) => self.error = Some(e),
                }
            }
        }

        // initial load
        if self.projects.is_empty() && !self.loading && self.error.is_none() {
            self.fetch(client);
        }

        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("Projects").color(palette::PURPLE));
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
            ui.add_space(8.0);
        }

        if self.show_create {
            egui::Frame::new()
                .fill(palette::SURFACE)
                .inner_margin(egui::Margin::same(12))
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("New project").color(palette::CYAN).strong());
                    ui.add_space(4.0);
                    egui::Grid::new("create_project_form").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(&mut self.name_buf);
                        ui.end_row();
                        ui.label("Slug");
                        ui.text_edit_singleline(&mut self.slug_buf);
                        ui.end_row();
                        ui.label("Description");
                        ui.text_edit_singleline(&mut self.desc_buf);
                        ui.end_row();
                    });
                    ui.add_space(4.0);
                    if ui.button(egui::RichText::new("Create").color(palette::GREEN)).clicked()
                        && !self.name_buf.is_empty()
                        && !self.slug_buf.is_empty()
                    {
                        self.create(client);
                    }
                });
            ui.add_space(8.0);
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for project in &self.projects {
                egui::Frame::new()
                    .fill(palette::SURFACE)
                    .inner_margin(egui::Margin::same(10))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&project.name).color(palette::FG).strong());
                            ui.label(
                                egui::RichText::new(format!("  #{}", project.slug))
                                    .color(palette::MUTED)
                                    .size(11.0),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("Open →").clicked() {
                                    *nav = View::Sources {
                                        project_slug: project.slug.clone(),
                                    };
                                }
                            });
                        });
                        if !project.description.is_empty() {
                            ui.label(
                                egui::RichText::new(&project.description).color(palette::MUTED).size(12.0),
                            );
                        }
                    });
                ui.add_space(4.0);
            }
        });
    }
}
