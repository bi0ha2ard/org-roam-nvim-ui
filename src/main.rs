mod graph;
mod history;

use eframe::egui;
use graph::{Graph, Link, Node, NodeDetails, NodeId, load_graph};
use history::History;
use itertools::Itertools;
use nalgebra::{Point2, Similarity2, Vector2, clamp};
use rand::SeedableRng;

type Point = Point2<f32>;
type Vector = Vector2<f32>;

#[derive(Default, Eq, PartialEq, PartialOrd, Ord, Copy, Clone, Hash)]
pub struct SubgNodeId(usize);

pub struct SubgLink {
    pub from: SubgNodeId,
    pub to: SubgNodeId,
}

struct PlacedNode {
    p: Point,  // Location
    f: Vector, // yet to be applied force
    layout_id: SubgNodeId,
    graph_node_id: NodeId, // keep track of id in case we're filtered down later
}

struct GraphLayout {
    // Current positions
    nodes: Vec<PlacedNode>,
    links: Vec<SubgLink>,
    backlinks: Vec<SubgLink>,
    rng: rand::rngs::StdRng,
    to_screen: Similarity2<f32>,
}

impl GraphLayout {
    // New layout for nodes
    fn new<'a, It>(nodes: It, graph: &Graph, seed: u64) -> GraphLayout
    where
        It: Iterator<Item = &'a Node>,
    {
        let positioned_nodes: Vec<PlacedNode> = nodes
            .enumerate()
            .map(|(id, n)| PlacedNode {
                p: Point::origin(),
                f: Vector::zeros(),
                layout_id: SubgNodeId(id),
                graph_node_id: n.id,
            })
            .collect();
        // TODO: factor out slicing of the graph?
        let to_subgraph_id = |target: NodeId| -> Option<SubgNodeId> {
            positioned_nodes
                .iter()
                .find_position(|n| n.graph_node_id == target)
                .map(|(id, _)| SubgNodeId(id))
        };
        let to_subgraph_link =
            |link: &Link| match (to_subgraph_id(link.from), to_subgraph_id(link.to)) {
                (Some(from), Some(to)) => Some(SubgLink { from, to }),
                _ => None,
            };
        let links = graph.links().flat_map(to_subgraph_link).collect();
        let backlinks = graph.backlinks().flat_map(to_subgraph_link).collect();

        GraphLayout {
            nodes: positioned_nodes,
            links,
            backlinks,
            rng: rand::rngs::StdRng::seed_from_u64(seed),
            to_screen: Similarity2::identity(),
        }
    }

    fn len_to_screen(&self, length: f32) -> f32 {
        self.to_screen.scaling() * length
    }

    fn pt_to_screen(&self, p: Point) -> egui::Pos2 {
        let p_screen = self.to_screen * p;
        egui::Pos2 {
            x: p_screen.x,
            y: p_screen.y,
        }
    }

    fn node_screen_pos(&self, SubgNodeId(node): SubgNodeId) -> egui::Pos2 {
        self.pt_to_screen(self.nodes.get(node).expect("node in subgraph").p)
    }

    /// Checks connectedness based on sub-graph indices
    fn is_connected(&self, from: SubgNodeId, to: SubgNodeId) -> bool {
        self.links
            .iter()
            .any(|l| (l.from == from && l.to == to) || (l.from == to && l.to == from))
    }

    fn by_real_id(&self, actual_id: NodeId) -> Option<&PlacedNode> {
        self.nodes.iter().find(|n| n.graph_node_id == actual_id)
    }

    fn tick(&mut self, dt: f32, force_min: f32, force_max: f32) -> bool {
        const MIN_DIST_FOR_DIR: f32 = 1e-6;
        const LINK_FORCE_MULT: f32 = 4.0;
        const DIST_FOR_LINKS: f32 = 10.0 * 10.0;
        const MAX_DIST_FOR_FORCE: f32 = 20.0 * 20.0;
        let distribution = rand::distributions::Uniform::<f32>::new_inclusive(-1.0, 1.0);
        for n in &mut self.nodes {
            n.f = Vector::zeros();
        }
        for (me, other) in (0..self.nodes.len()).tuple_combinations() {
            let displacement = {
                let me = self.nodes.get(me).unwrap();
                let other = self.nodes.get(other).unwrap();
                other.p - me.p
            };
            let dist = displacement.norm_squared();
            let dir: Vector = if dist.abs() < MIN_DIST_FOR_DIR {
                Vector::from_distribution(&distribution, &mut self.rng).normalize()
            } else {
                displacement.normalize()
            };
            let clamped_dist = clamp(dist, 0.01, 1e10);
            let is_connected = self.is_connected(SubgNodeId(me), SubgNodeId(other));
            if dist > DIST_FOR_LINKS && is_connected {
                // Attracting
                let f = LINK_FORCE_MULT * dt * dir.scale(1. / clamped_dist.min(9.));
                self.nodes[other].f -= f;
                self.nodes[me].f += f;
            }
            if dist > MAX_DIST_FOR_FORCE {
                continue;
            }
            if is_connected && dist <= DIST_FOR_LINKS + 1. && dist > DIST_FOR_LINKS - 1. {
                continue;
            }
            // Repelling
            self.nodes[other].f += dt * dir.scale(1. / clamped_dist);
            self.nodes[me].f += dt * dir.scale(-1. / clamped_dist);
        }
        let mut skipped = 0;
        for n in &mut self.nodes {
            let len = n.f.norm().min(force_max);
            if len < force_min {
                skipped += 1;
                continue;
            }
            n.p += n.f.normalize() * len;
        }
        skipped == self.nodes.len()
    }
}

struct GraphViewState {
    dt: f32,
    force_min: f32,
    force_max: f32,
    zoom: f32,
    offset: egui::Vec2,
}

impl Default for GraphViewState {
    fn default() -> Self {
        Self {
            dt: 0.2,
            force_min: 0.002,
            force_max: 1.0,
            zoom: 1.,
            offset: egui::Vec2::default(),
        }
    }
}

impl GraphViewState {
    fn text_alpha(&self) -> f32 {
        ((self.zoom - 10.) / 20.).clamp(0., 1.)
    }
}

struct Filter {
    title: String,
    show_connected: bool,
}

struct RoamUI {
    graph: Graph,
    layout: GraphLayout,
    history: History<NodeId>,
    view_state: GraphViewState,
    filter: Filter,
    selected: Option<NodeDetails>,
    highlighted: Option<NodeId>,
}

const RADIUS: f32 = 1.0;

impl RoamUI {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // customization here
        let graph = load_graph();
        let layout = GraphLayout::new(graph.nodes(), &graph, 0);
        Self {
            graph,
            layout,
            history: History::default(),
            view_state: GraphViewState::default(),
            filter: Filter {
                title: String::new(),
                show_connected: true,
            },
            selected: None,
            highlighted: None,
        }
    }

    fn apply_filter(&mut self) {
        let title = self.filter.title.as_str();
        if title.is_empty() {
            self.layout = GraphLayout::new(self.graph.nodes(), &self.graph, 0);
        } else {
            let case_sensitive = title.chars().any(|c| c.is_uppercase());
            let lower_title = title.to_lowercase();
            let matcher = |n: &&Node| {
                if case_sensitive {
                    n.title.contains(title)
                } else {
                    n.title.to_lowercase().contains(lower_title.as_str())
                }
            };
            if !self.filter.show_connected {
                self.layout = GraphLayout::new(self.graph.nodes().filter(matcher), &self.graph, 0);
            } else {
                // TODO: 2-stage hilighting: direct and connected nodes
                self.layout = GraphLayout::new(
                    self.graph
                        .nodes()
                        .filter(matcher)
                        .flat_map(|n| self.graph.bfs(n.id))
                        .unique_by(|n| n.id),
                    &self.graph,
                    0,
                );
            }
        }
    }

    fn render_filter(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("filter")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label("Filter");
                    let was_empty = self.filter.title.is_empty();
                    let title_changed = ui
                        .add(egui::TextEdit::singleline(&mut self.filter.title))
                        .changed();
                    let checkbox_changed = ui
                        .checkbox(&mut self.filter.show_connected, "include connected nodes")
                        .changed();
                    if was_empty && self.filter.title.is_empty() {
                        // Prevent re-render if filter hasn't changed
                        return;
                    }
                    if title_changed || checkbox_changed {
                        self.apply_filter();
                    }
                });
            });
    }

    fn node_title_in_graph(
        &self,
        painter: &egui::Painter,
        node: &PlacedNode,
        offs: &egui::Vec2,
        text_color: egui::Color32,
    ) {
        painter.text(
            self.layout
                .pt_to_screen(node.p - Vector::new(0.0, 1.0) * (RADIUS + 0.5))
                + *offs,
            egui::Align2::CENTER_CENTER,
            &self.graph.node(node.graph_node_id).unwrap().title,
            egui::FontId::default(),
            text_color,
        );
    }

    fn render_selected(&self, ctx: &egui::Context) -> (Option<NodeId>, Option<NodeId>) {
        let Some(details) = &self.selected else {
            return (None, self.highlighted);
        };
        let mut clicked = None;
        let mut highlighted = None;
        let node = self.graph.node(details.node).expect("node exists");

        let mut render_link = |ui: &mut egui::Ui, (id, text): &(NodeId, String)| {
            let mut l = ui.label(text);
            if matches!(self.highlighted, Some(hl_id) if hl_id == *id) {
                l = l.highlight();
            }
            if l.clicked() {
                clicked = Some(*id);
            }
            if l.contains_pointer() {
                highlighted = Some(*id);
            }
        };

        egui::SidePanel::right("selected")
            .exact_width(200.)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(&node.title);
                        ui.separator();
                        // ui.label(format!(
                        //     "ID: {}, UUID: {}, links: {}, backlinks: {}",
                        //     node.id,
                        //     node.uuid,
                        //     details.links.len(),
                        //     details.backlinks.len(),
                        // ));
                        // ui.separator();
                        ui.label("Links");
                        for l in &details.links {
                            render_link(ui, l);
                        }
                        ui.separator();
                        ui.label("Backlinks");
                        for l in &details.backlinks {
                            render_link(ui, l);
                        }
                    })
                });
            });
        (clicked, highlighted)
    }

    fn select_node(&mut self, node: NodeId) {
        self.selected = Some(self.graph.node_details(node));
        self.history.push(Some(node));
    }

    fn deselect_node(&mut self) {
        self.selected = None;
        self.history.push(None);
    }

    fn back(&mut self) {
        if let Some(node) = self.history.pop() {
            self.selected = Some(self.graph.node_details(node));
        }
    }

    fn fwd(&mut self) {
        if let Some(node) = self.history.unpop() {
            self.selected = Some(self.graph.node_details(node));
        }
    }

    fn handle_global_shortcuts(&mut self, input: &egui::InputState) {
        if input.pointer.button_pressed(egui::PointerButton::Extra1) {
            self.back();
        }
        if input.pointer.button_pressed(egui::PointerButton::Extra2) {
            self.fwd();
        }
    }

    fn draw_links(&self, painter: &egui::Painter, offs: egui::Vec2) {
        let alpha = if self.selected.is_some() { 0.5 } else { 1.0 };
        let regular_stroke = egui::Stroke::new(alpha, egui::Color32::YELLOW);

        // All links
        for l in &self.layout.links {
            let left = self.layout.node_screen_pos(l.from);
            let right = self.layout.node_screen_pos(l.to);
            painter.line_segment([left + offs, right + offs], regular_stroke);
        }

        // Network around selected node
        // TODO: also hilight non-direct edges/nodes better
        // TODO: pre-compute this on selection?
        if let Some((selection, in_layout)) = self.selected.as_ref().and_then(|s| {
            if let Some(in_layout) = self.layout.by_real_id(s.node) {
                return Some((s, in_layout));
            }
            None
        }) {
            let link_stroke = egui::Stroke::new(1.0, egui::Color32::RED);
            let backlink_stroke = egui::Stroke::new(1.0, egui::Color32::MAGENTA);
            for other_graph in self.graph.direct_links(selection.node) {
                if let Some(other_placed) = self.layout.by_real_id(other_graph.id) {
                    let left = self.layout.node_screen_pos(in_layout.layout_id);
                    let right = self.layout.node_screen_pos(other_placed.layout_id);
                    painter.line_segment([left + offs, right + offs], link_stroke);
                }
            }
            for other_graph in self.graph.direct_backlinks(selection.node) {
                if let Some(other_placed) = self.layout.by_real_id(other_graph.id) {
                    let left = self.layout.node_screen_pos(other_placed.layout_id);
                    let right = self.layout.node_screen_pos(in_layout.layout_id);
                    painter.line_segment([left + offs, right + offs], backlink_stroke);
                }
            }
        }
    }

    fn render_graph(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut clicked = false;
            if ui.ui_contains_pointer() {
                ui.input(|i| {
                    clicked = i.pointer.primary_clicked();
                    if i.smooth_scroll_delta.y != 0. {
                        self.view_state.zoom = (self.view_state.zoom
                            + (self.view_state.zoom / 400.).clamp(0., 0.1)
                                * i.smooth_scroll_delta.y)
                            .clamp(0.1, 400.0);
                    }
                    if i.pointer.primary_down() && i.pointer.is_decidedly_dragging() {
                        self.view_state.offset += i.pointer.delta()
                    }
                });
            }
            // TODO: zoom to mouse pos
            self.layout.to_screen.set_scaling(self.view_state.zoom);
            self.layout.to_screen.isometry.translation.x = self.view_state.offset.x;
            self.layout.to_screen.isometry.translation.y = self.view_state.offset.y;

            let offs = ctx.input(|i| i.viewport().inner_rect).map(|r| r.max - r.min).unwrap_or_else(||ui.min_size()) / 2.0;
            let painter = ui.painter();
            let settled = self.layout.tick(
                self.view_state.dt,
                self.view_state.force_min,
                self.view_state.force_max,
            );
            let mut hovered_node = None;
            let radius_screen = self.layout.len_to_screen(RADIUS);

            self.draw_links(painter, offs);

            for n in &self.layout.nodes {
                let pos = self.layout.pt_to_screen(n.p) + offs;
                let mouse_over = ctx
                    .pointer_latest_pos()
                    .map(|p| (p - pos).length_sq() <= radius_screen * radius_screen)
                    .unwrap_or(false);
                painter.circle_filled(
                    pos,
                    radius_screen,
                    if mouse_over || matches!(&self.highlighted, Some(id) if *id == n.graph_node_id) {
                        egui::Color32::BLUE
                    } else if matches!(&self.selected.as_ref().map(|n|n.node), Some(id) if *id == n.graph_node_id) {
                        egui::Color32::ORANGE
                    } else {
                        egui::Color32::RED
                    },
                );
                if mouse_over {
                    hovered_node = Some(n.graph_node_id);
                    self.highlighted = hovered_node;
                }
            }
            if clicked && hovered_node.map(|n| self.select_node(n)).is_none() {
                self.deselect_node();
            }
            let text_alpha = self.view_state.text_alpha();
            let text_color =
                egui::Color32::from_rgba_unmultiplied(128, 128, 128, (text_alpha * 255.) as u8);
            for n in &self.layout.nodes {
                if matches!(&self.selected.as_ref().map(|n|n.node), Some(id) if *id == n.graph_node_id) {
                    self.node_title_in_graph(painter, n, &offs, egui::Color32::ORANGE);
                } else if text_alpha > 0. {
                    self.node_title_in_graph(painter, n, &offs, text_color);
                }
            }
            if ui.ui_contains_pointer() {
                if let Some(id) = hovered_node {
                    let graph_node = self.graph.node(id).unwrap();
                    egui::show_tooltip_at_pointer(
                        ctx,
                        painter.layer_id(),
                        egui::Id::new("title"),
                        |ui| {
                            let label =
                            egui::Label::new(&graph_node.title).wrap_mode(egui::TextWrapMode::Extend);
                            ui.add(label);
                        },
                    );
                }
            }
            if !settled {
                ctx.request_repaint();
            }
        });
    }
}

impl eframe::App for RoamUI {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Q)) {
            ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Close);
        }
        egui::SidePanel::left("Graph Settings")
            .resizable(false)
            .show(ctx, |ui| {
                ui.add(egui::Slider::new(&mut self.view_state.dt, 0.001_f32..=5.));
                ui.add(egui::Slider::new(
                    &mut self.view_state.force_max,
                    1_f32..=10.,
                ));
                ui.add(egui::Slider::new(
                    &mut self.view_state.force_min,
                    0_f32..=1.,
                ));
                if ui.button("Reset").clicked() {
                    for n in &mut self.layout.nodes {
                        n.p = Point::origin();
                    }
                }
            });
        self.render_filter(ctx);
        let (next_sel, next_hl) = self.render_selected(ctx);
        if let Some(next_selection) = next_sel {
            self.select_node(next_selection);
        }
        self.highlighted = next_hl;
        self.render_graph(ctx);
        ctx.input(|i| self.handle_global_shortcuts(i));
    }
}

fn main() {
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "org-roam-nvim-ui",
        native_options,
        Box::new(|cc| Ok(Box::new(RoamUI::new(cc)))),
    );
}
