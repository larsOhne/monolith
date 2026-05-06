use std::sync::Arc;

use crate::{
    api::Client,
    views::{drift, evidence, graph, projects, sources, statements},
    worker::{WorkerHandle, WorkerState},
};

// ─── Dracula palette ────────────────────────────────────────────────────────

pub mod palette {
    use egui::Color32;
    pub const CANVAS: Color32 = Color32::from_rgb(0x13, 0x14, 0x1C);
    pub const BG: Color32 = Color32::from_rgb(0x28, 0x2A, 0x36);
    pub const SURFACE: Color32 = Color32::from_rgb(0x44, 0x47, 0x5A);
    pub const MUTED: Color32 = Color32::from_rgb(0x62, 0x72, 0xA4);
    pub const FG: Color32 = Color32::from_rgb(0xF8, 0xF8, 0xF2);
    pub const PINK: Color32 = Color32::from_rgb(0xFF, 0x79, 0xC6);
    pub const PURPLE: Color32 = Color32::from_rgb(0xBD, 0x93, 0xF9);
    pub const CYAN: Color32 = Color32::from_rgb(0x8B, 0xE9, 0xFD);
    pub const GREEN: Color32 = Color32::from_rgb(0x50, 0xFA, 0x7B);
    pub const YELLOW: Color32 = Color32::from_rgb(0xF1, 0xFA, 0x8C);
    pub const ORANGE: Color32 = Color32::from_rgb(0xFF, 0xB8, 0x6C);
    pub const RED: Color32 = Color32::from_rgb(0xFF, 0x55, 0x55);
}

// ─── Navigation ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub enum View {
    #[default]
    Projects,
    Sources { project_slug: String },
    Evidence { project_slug: String },
    Statements { project_slug: String },
    Graph { project_slug: String },
    Drift { project_slug: String },
}

// ─── App ────────────────────────────────────────────────────────────────────

pub struct MonolithApp {
    worker: Arc<WorkerHandle>,
    client: Option<Client>,
    current_view: View,

    // per-view state
    pub projects: projects::ProjectsView,
    pub sources: sources::SourcesView,
    pub evidence: evidence::EvidenceView,
    pub statements: statements::StatementsView,
    pub graph: graph::GraphView,
    pub drift: drift::DriftView,

    // drift badge count (polled in background)
    pub drift_count: Arc<std::sync::Mutex<usize>>,
    last_drift_poll: std::time::Instant,
}

impl MonolithApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let worker = crate::worker::spawn_worker();
        Self {
            worker,
            client: None,
            current_view: View::default(),
            projects: projects::ProjectsView::default(),
            sources: sources::SourcesView::default(),
            evidence: evidence::EvidenceView::default(),
            statements: statements::StatementsView::default(),
            graph: graph::GraphView::default(),
            drift: drift::DriftView::default(),
            drift_count: Arc::new(std::sync::Mutex::new(0)),
            last_drift_poll: std::time::Instant::now()
                - std::time::Duration::from_secs(300), // poll immediately
        }
    }

    fn ensure_client(&mut self) {
        if self.client.is_none() {
            if let Some(url) = self.worker.base_url() {
                self.client = Some(Client::new(&url));
            }
        }
    }

    pub fn navigate(&mut self, view: View) {
        self.current_view = view;
    }
}

fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = palette::BG;
    visuals.window_fill = palette::BG;
    visuals.faint_bg_color = palette::CANVAS;
    visuals.extreme_bg_color = palette::CANVAS;
    visuals.code_bg_color = palette::SURFACE;
    visuals.override_text_color = Some(palette::FG);
    visuals.selection.bg_fill = palette::PURPLE.gamma_multiply(0.4);
    visuals.selection.stroke.color = palette::PURPLE;
    visuals.widgets.noninteractive.bg_fill = palette::SURFACE;
    visuals.widgets.noninteractive.fg_stroke.color = palette::MUTED;
    visuals.widgets.inactive.bg_fill = palette::SURFACE;
    visuals.widgets.inactive.fg_stroke.color = palette::FG;
    visuals.widgets.hovered.bg_fill = palette::MUTED;
    visuals.widgets.hovered.fg_stroke.color = palette::FG;
    visuals.widgets.active.bg_fill = palette::PURPLE;
    visuals.widgets.active.fg_stroke.color = palette::BG;
    ctx.set_visuals(visuals);
}

impl eframe::App for MonolithApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_theme(ctx);
        self.ensure_client();

        // ── Splash / loading state ──────────────────────────────────────────
        let state = self.worker.state.lock().unwrap().clone();
        match state {
            WorkerState::Starting => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(200.0);
                        ui.heading(egui::RichText::new("Monolith").color(palette::PURPLE).size(36.0));
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("Starting Python worker…").color(palette::MUTED));
                        ui.add_space(8.0);
                        ui.spinner();
                    });
                });
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
                return;
            }
            WorkerState::Failed(ref msg) => {
                let msg = msg.clone();
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(200.0);
                        ui.heading(
                            egui::RichText::new("Worker failed to start").color(palette::RED).size(24.0),
                        );
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(&msg).color(palette::ORANGE));
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new(
                            "Make sure `python3 -m monolith` is available in your PATH.\nThen restart Monolith."
                        ).color(palette::MUTED));
                    });
                });
                return;
            }
            WorkerState::Stopped => return,
            WorkerState::Ready { .. } => {}
        }

        // ── Background drift poll (every 5 min) ────────────────────────────
        if let (Some(client), Some(slug)) = (
            self.client.clone(),
            self.active_project_slug(),
        ) {
            if self.last_drift_poll.elapsed() > std::time::Duration::from_secs(300) {
                self.last_drift_poll = std::time::Instant::now();
                let count_arc = self.drift_count.clone();
                std::thread::spawn(move || {
                    if let Ok(report) = client.check_drift(&slug) {
                        let broken = report.entries.iter().filter(|e| e.status != "valid").count();
                        *count_arc.lock().unwrap() = broken;
                    }
                });
            }
        }

        // ── Top bar ────────────────────────────────────────────────────────
        self.render_top_bar(ctx);

        // ── Side nav ──────────────────────────────────────────────────────
        egui::SidePanel::left("nav")
            .resizable(false)
            .exact_width(160.0)
            .show(ctx, |ui| {
                self.render_nav(ui);
            });

        // ── Status bar ────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("status")
            .exact_height(24.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(
                        egui::RichText::new("● worker running").color(palette::GREEN).size(11.0),
                    );
                    if let Some(url) = self.worker.base_url() {
                        ui.separator();
                        ui.label(egui::RichText::new(&url).color(palette::MUTED).size(11.0));
                    }
                });
            });

        // ── Main panel ────────────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            let client = match self.client.clone() {
                Some(c) => c,
                None => {
                    ui.label("Connecting…");
                    return;
                }
            };
            match self.current_view.clone() {
                View::Projects => self.projects.show(ui, &client, &mut self.current_view),
                View::Sources { project_slug } => {
                    self.sources.show(ui, &client, &project_slug, &mut self.current_view)
                }
                View::Evidence { project_slug } => {
                    self.evidence.show(ui, &client, &project_slug, &mut self.current_view)
                }
                View::Statements { project_slug } => {
                    self.statements.show(ui, &client, &project_slug, &mut self.current_view)
                }
                View::Graph { project_slug } => {
                    self.graph.show(ui, &client, &project_slug)
                }
                View::Drift { project_slug } => {
                    self.drift.show(ui, &client, &project_slug, &mut self.drift_count)
                }
            }
        });
    }
}

impl MonolithApp {
    fn active_project_slug(&self) -> Option<String> {
        match &self.current_view {
            View::Sources { project_slug }
            | View::Evidence { project_slug }
            | View::Statements { project_slug }
            | View::Graph { project_slug }
            | View::Drift { project_slug } => Some(project_slug.clone()),
            _ => None,
        }
    }

    fn render_top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("topbar")
            .exact_height(40.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(
                        egui::RichText::new("◆ Monolith").color(palette::PURPLE).size(16.0).strong(),
                    );

                    if let Some(slug) = self.active_project_slug() {
                        ui.separator();
                        ui.label(egui::RichText::new(&slug).color(palette::CYAN));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Drift badge
                        let count = *self.drift_count.lock().unwrap();
                        if count > 0 {
                            let label = format!("⚠ Drift {count}");
                            if ui
                                .button(egui::RichText::new(&label).color(palette::ORANGE))
                                .clicked()
                            {
                                if let Some(slug) = self.active_project_slug() {
                                    self.current_view = View::Drift { project_slug: slug };
                                }
                            }
                        }
                    });
                });
            });
    }

    fn render_nav(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.label(egui::RichText::new("NAVIGATION").color(palette::MUTED).size(10.0));
        ui.add_space(4.0);

        let nav_btn = |ui: &mut egui::Ui, label: &str, active: bool| -> bool {
            let text = if active {
                egui::RichText::new(label).color(palette::PURPLE).strong()
            } else {
                egui::RichText::new(label).color(palette::FG)
            };
            ui.add_sized([150.0, 28.0], egui::Button::new(text).frame(false)).clicked()
        };

        if nav_btn(ui, "⊞  Projects", matches!(self.current_view, View::Projects)) {
            self.current_view = View::Projects;
        }

        if let Some(slug) = self.active_project_slug() {
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(egui::RichText::new("PROJECT").color(palette::MUTED).size(10.0));
            ui.add_space(4.0);

            if nav_btn(
                ui,
                "⊟  Sources",
                matches!(&self.current_view, View::Sources { .. }),
            ) {
                self.current_view = View::Sources { project_slug: slug.clone() };
            }
            if nav_btn(
                ui,
                "◈  Evidence",
                matches!(&self.current_view, View::Evidence { .. }),
            ) {
                self.current_view = View::Evidence { project_slug: slug.clone() };
            }
            if nav_btn(
                ui,
                "◉  Statements",
                matches!(&self.current_view, View::Statements { .. }),
            ) {
                self.current_view = View::Statements { project_slug: slug.clone() };
            }
            if nav_btn(
                ui,
                "◎  Graph",
                matches!(&self.current_view, View::Graph { .. }),
            ) {
                self.current_view = View::Graph { project_slug: slug.clone() };
            }
            if nav_btn(
                ui,
                "⚠  Drift",
                matches!(&self.current_view, View::Drift { .. }),
            ) {
                self.current_view = View::Drift { project_slug: slug.clone() };
            }
        }
    }
}
