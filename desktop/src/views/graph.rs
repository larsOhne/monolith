/// Graph view — force-directed layout running in a background thread,
/// rendered via egui's `Painter` (immediate mode, GPU-accelerated via wgpu backend).
use std::sync::{mpsc, Arc, Mutex};

use egui::{Color32, Pos2, Rect, Vec2};
use petgraph::{graph::NodeIndex, Graph};

use crate::api::{Client, Evidence, Project, Source, Statement};
use crate::app::palette;

// ─── Domain graph ────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum NodeKind {
    Source(Source),
    Evidence(Evidence),
    Statement(Statement),
}

impl NodeKind {
    pub fn label(&self) -> &str {
        match self {
            NodeKind::Source(s) => &s.path,
            NodeKind::Evidence(e) => &e.verbatim_text,
            NodeKind::Statement(s) => &s.content,
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            NodeKind::Source(_) => palette::FG,
            NodeKind::Evidence(e) => match e.status.as_str() {
                "drifted" => palette::ORANGE,
                "broken" => palette::RED,
                _ => palette::CYAN,
            },
            NodeKind::Statement(_) => palette::PURPLE,
        }
    }
}

// ─── Layout ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct LayoutNode {
    pos: Pos2,
    vel: Vec2,
}

struct LayoutState {
    graph: Graph<NodeKind, ()>,
    nodes: Vec<LayoutNode>,
}

impl LayoutState {
    fn new(graph: Graph<NodeKind, ()>) -> Self {
        let n = graph.node_count();
        // Place nodes in a circle initially
        let nodes = (0..n)
            .map(|i| {
                let angle = (i as f32 / n as f32) * std::f32::consts::TAU;
                let r = 200.0_f32;
                LayoutNode {
                    pos: Pos2::new(r * angle.cos(), r * angle.sin()),
                    vel: Vec2::ZERO,
                }
            })
            .collect();
        Self { graph, nodes }
    }

    /// Fruchterman-Reingold iteration (one step)
    fn step(&mut self) {
        let n = self.nodes.len();
        if n == 0 {
            return;
        }

        let k = 100.0_f32; // ideal spring length
        let mut forces = vec![Vec2::ZERO; n];

        // Repulsion between all pairs
        for i in 0..n {
            for j in (i + 1)..n {
                let delta = self.nodes[i].pos - self.nodes[j].pos;
                let dist = delta.length().max(0.1);
                let force = (k * k / dist) * (delta / dist);
                forces[i] += force;
                forces[j] -= force;
            }
        }

        // Attraction along edges
        for edge in self.graph.raw_edges() {
            let i = edge.source().index();
            let j = edge.target().index();
            if i < n && j < n {
                let delta = self.nodes[j].pos - self.nodes[i].pos;
                let dist = delta.length().max(0.1);
                let force = (dist * dist / k) * (delta / dist);
                forces[i] += force;
                forces[j] -= force;
            }
        }

        // Gravity towards centre
        for (i, node) in self.nodes.iter_mut().enumerate() {
            let gravity = -node.pos.to_vec2() * 0.01;
            node.vel = (node.vel + forces[i] + gravity) * 0.85; // damping
            node.pos += node.vel * 0.016; // dt ≈ 16 ms
        }
    }
}

// ─── Background layout thread ─────────────────────────────────────────────

type SharedPositions = Arc<Mutex<Vec<(Pos2, NodeKind)>>>;

fn start_layout_thread(
    graph: Graph<NodeKind, ()>,
    positions: SharedPositions,
) {
    std::thread::spawn(move || {
        let mut layout = LayoutState::new(graph);
        loop {
            for _ in 0..4 {
                layout.step();
            }
            let snapshot: Vec<(Pos2, NodeKind)> = layout
                .graph
                .node_indices()
                .map(|idx| (layout.nodes[idx.index()].pos, layout.graph[idx].clone()))
                .collect();
            *positions.lock().unwrap() = snapshot;
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    });
}

// ─── API load ────────────────────────────────────────────────────────────────

enum Msg {
    GraphData {
        sources: Vec<Source>,
        evidence: Vec<crate::api::Evidence>,
        statements: Vec<Statement>,
    },
    Error(String),
}

// ─── View ────────────────────────────────────────────────────────────────────

pub struct GraphView {
    loaded_for: Option<String>,
    loading: bool,
    error: Option<String>,
    rx: Option<mpsc::Receiver<Msg>>,

    positions: SharedPositions,
    layout_started: bool,

    // pan/zoom
    pan: Vec2,
    zoom: f32,

    // hover
    hovered: Option<String>, // node label
}

impl Default for GraphView {
    fn default() -> Self {
        Self {
            loaded_for: None,
            loading: false,
            error: None,
            rx: None,
            positions: Arc::new(Mutex::new(Vec::new())),
            layout_started: false,
            pan: Vec2::ZERO,
            zoom: 1.0,
            hovered: None,
        }
    }
}

impl GraphView {
    fn fetch(&mut self, client: &Client, slug: &str) {
        let (tx, rx) = mpsc::channel::<Msg>();
        self.rx = Some(rx);
        self.loading = true;
        self.error = None;
        self.loaded_for = Some(slug.to_string());
        self.positions = Arc::new(Mutex::new(Vec::new()));
        self.layout_started = false;
        let c = client.clone();
        let s = slug.to_string();
        std::thread::spawn(move || {
            let sources = c.list_sources(&s).unwrap_or_default();
            let statements = c.list_statements(&s).unwrap_or_default();
            // gather evidence via search (empty query returns all)
            let evidence = c.search_evidence("").unwrap_or_default();
            let _ = tx.send(Msg::GraphData { sources, evidence, statements });
        });
    }

    fn build_graph(
        sources: Vec<Source>,
        evidence: Vec<crate::api::Evidence>,
        statements: Vec<Statement>,
    ) -> Graph<NodeKind, ()> {
        let mut g: Graph<NodeKind, ()> = Graph::new();

        // Source nodes
        let src_map: std::collections::HashMap<String, NodeIndex> = sources
            .into_iter()
            .map(|s| {
                let id = s.id.clone();
                let idx = g.add_node(NodeKind::Source(s));
                (id, idx)
            })
            .collect();

        // Evidence nodes + edge from source
        let ev_map: std::collections::HashMap<String, NodeIndex> = evidence
            .into_iter()
            .map(|e| {
                let src_id = e.source_id.clone();
                let ev_id = e.id.clone();
                let idx = g.add_node(NodeKind::Evidence(e));
                if let Some(&src_idx) = src_map.get(&src_id) {
                    g.add_edge(src_idx, idx, ());
                }
                (ev_id, idx)
            })
            .collect();

        // Statement nodes + edges from evidence
        for stmt in statements {
            let ev_ids = stmt.evidence_ids.clone();
            let stmt_idx = g.add_node(NodeKind::Statement(stmt));
            for ev_id in &ev_ids {
                if let Some(&ev_idx) = ev_map.get(ev_id) {
                    g.add_edge(ev_idx, stmt_idx, ());
                }
            }
        }

        g
    }

    pub fn show(&mut self, ui: &mut egui::Ui, client: &Client, slug: &str) {
        if self.loaded_for.as_deref() != Some(slug) {
            self.fetch(client, slug);
        }

        if let Some(ref rx) = self.rx {
            while let Ok(msg) = rx.try_recv() {
                self.loading = false;
                match msg {
                    Msg::GraphData { sources, evidence, statements } => {
                        let graph = Self::build_graph(sources, evidence, statements);
                        let positions = self.positions.clone();
                        if !self.layout_started {
                            self.layout_started = true;
                            start_layout_thread(graph, positions);
                        }
                    }
                    Msg::Error(e) => self.error = Some(e),
                }
            }
        }

        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("Knowledge Graph").color(palette::PURPLE));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("↻ Reload").clicked() {
                    self.loaded_for = None;
                }
                ui.label(
                    egui::RichText::new("scroll = zoom  •  drag = pan")
                        .color(palette::MUTED)
                        .size(10.0),
                );
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

        // ── Canvas ───────────────────────────────────────────────────────
        let available = ui.available_rect_before_wrap();
        let (resp, painter) = ui.allocate_painter(available.size(), egui::Sense::click_and_drag());

        // Pan
        if resp.dragged() {
            self.pan += resp.drag_delta();
        }
        // Zoom
        if let Some(hover_pos) = resp.hover_pos() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let factor = (scroll * 0.001).exp();
                let before = (hover_pos - available.min - self.pan) / self.zoom;
                self.zoom = (self.zoom * factor).clamp(0.1, 10.0);
                let after = (hover_pos - available.min - self.pan) / self.zoom;
                self.pan += (after - before) * self.zoom;
            }
        }

        let centre = available.min + available.size() / 2.0;
        let world_to_screen = |p: Pos2| -> Pos2 {
            centre + (p.to_vec2() * self.zoom) + self.pan
        };

        let positions = self.positions.lock().unwrap().clone();
        self.hovered = None;

        // Draw edges (need two passes since we don't store the graph here)
        // We just draw nodes — edges would need the graph topology.
        // For a complete implementation the layout thread would emit (pos, kind, edges).
        // Here we draw nodes with labels; the force-directed clusters make topology implicit.

        let node_radius = 8.0_f32 * self.zoom.sqrt();
        let hover_pos = ui.input(|i| i.pointer.hover_pos());

        for (world_pos, kind) in &positions {
            let screen_pos = world_to_screen(*world_pos);

            if !available.contains(screen_pos) {
                continue;
            }

            let color = kind.color();

            painter.circle_filled(screen_pos, node_radius, color);
            painter.circle_stroke(
                screen_pos,
                node_radius,
                egui::Stroke::new(1.5, color.gamma_multiply(1.4)),
            );

            // Label (only if zoomed in enough)
            if self.zoom > 0.5 {
                let label = kind.label();
                let truncated: String = label.chars().take(40).collect();
                let display = if label.len() > 40 {
                    format!("{truncated}…")
                } else {
                    truncated
                };
                painter.text(
                    screen_pos + Vec2::new(node_radius + 4.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    &display,
                    egui::FontId::proportional(10.0 * self.zoom.min(1.5)),
                    palette::MUTED,
                );
            }

            // Hover detection
            if let Some(hp) = hover_pos {
                if (hp - screen_pos).length() < node_radius + 4.0 {
                    self.hovered = Some(kind.label().to_string());
                }
            }
        }

        // Hover tooltip
        if let Some(ref label) = self.hovered {
            if let Some(hp) = hover_pos {
                let tooltip_rect = Rect::from_min_size(
                    hp + Vec2::new(12.0, -8.0),
                    Vec2::new(300.0, 60.0),
                );
                painter.rect_filled(tooltip_rect, 4.0, palette::SURFACE);
                painter.rect_stroke(
                    tooltip_rect,
                    4.0,
                    egui::Stroke::new(1.0, palette::MUTED),
                    egui::StrokeKind::Middle,
                );
                painter.text(
                    tooltip_rect.min + Vec2::new(8.0, 8.0),
                    egui::Align2::LEFT_TOP,
                    label,
                    egui::FontId::proportional(11.0),
                    palette::FG,
                );
            }
        }

        // Legend
        let legend_pos = available.min + Vec2::new(8.0, 8.0);
        for (i, (label, color)) in [
            ("Source", palette::FG),
            ("Evidence (valid)", palette::CYAN),
            ("Evidence (drifted)", palette::ORANGE),
            ("Evidence (broken)", palette::RED),
            ("Statement", palette::PURPLE),
        ]
        .iter()
        .enumerate()
        {
            let y = legend_pos.y + i as f32 * 18.0;
            let dot = Pos2::new(legend_pos.x + 6.0, y + 6.0);
            painter.circle_filled(dot, 5.0, *color);
            painter.text(
                Pos2::new(dot.x + 12.0, dot.y),
                egui::Align2::LEFT_CENTER,
                *label,
                egui::FontId::proportional(10.0),
                palette::MUTED,
            );
        }

        // Request continuous repaint while layout is running
        if self.layout_started {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(32));
        }
    }
}
