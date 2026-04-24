mod commands;
mod graph;
mod history;
mod style;

use std::fmt::Display;

use eframe::egui;
use egui::Sense;
use graph::{Graph, Link, Node, NodeDetails, NodeId, load_graph};
use history::History;
use itertools::Itertools;
use nalgebra::{Point2, Similarity2, Vector2, clamp};
use rand::SeedableRng;
use rand::distr::Uniform;

use crate::commands::CommandIPC;
use crate::style::{GRUVBOX, graph_style, set_theme};

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

struct LayoutOptimizerParams {
    f_min: f32,
    f_max: f32, // TODO: remove. can probably just be 2x desired_dist or something
    decay_rate: f64,
    desired_dist: f32,
}

impl Default for LayoutOptimizerParams {
    fn default() -> Self {
        Self {
            f_min: 0.05,
            f_max: 80.0,
            decay_rate: 0.95,
            desired_dist: 10.0,
        }
    }
}

struct GraphLayout {
    // Current positions
    nodes: Vec<PlacedNode>,
    links: Vec<SubgLink>,
    #[allow(dead_code)]
    backlinks: Vec<SubgLink>,
    rng: rand::rngs::StdRng,
    to_screen: Similarity2<f32>,

    params: LayoutOptimizerParams,
    t_total: f64,
    curr_damping: f64,
    settled: bool,
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
        let links = graph.links().filter_map(to_subgraph_link).collect();
        let backlinks = graph.backlinks().filter_map(to_subgraph_link).collect();

        GraphLayout {
            nodes: positioned_nodes,
            links,
            backlinks,
            rng: rand::rngs::StdRng::seed_from_u64(seed),
            to_screen: Similarity2::identity(),
            params: LayoutOptimizerParams::default(),
            t_total: 0.0,
            curr_damping: 1.0,
            settled: false,
        }
    }

    fn set_params(&mut self, params: LayoutOptimizerParams) {
        self.params = params;
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

    fn reset_layout(&mut self) {
        for n in &mut self.nodes {
            n.p = Point::origin();
        }
        self.t_total = 0.0;
        self.curr_damping = 1.0;
        self.settled = false;
        // TODO: also reset the RNG?
    }

    /// To avoid oscillation, just run twice each timestep, much smoother that way.
    fn double_tick(&mut self, dt: f64) -> bool {
        self.tick(dt / 2.0) || self.tick(dt / 2.0)
    }

    /// simulate one tick
    /// Note: dt makes this frame-rate independent, but it also results in non-repeatable
    /// simulatino granularity, so maybe that's not a good idea anyway.
    fn tick(&mut self, dt: f64) -> bool {
        const T_UNDAMPED: f64 = 2.0;
        const T_SCALE: f64 = 10.;
        const MIN_DIST_FOR_DIR: f32 = 1e-6;

        if self.settled {
            return true;
        }
        let dt = dt * T_SCALE;

        let link_force_mult: f32 = self.params.desired_dist;
        let rep_force_mult: f32 = link_force_mult * link_force_mult;
        let max_dist_for_force: f32 = (2.0 * link_force_mult) * (2.0 * link_force_mult);

        let distribution = Uniform::<f32>::new_inclusive(-1.0, 1.0).expect("random distribution");

        for n in &mut self.nodes {
            n.f = Vector::zeros();
        }

        let get_displacement =
            move |me: &Point, other: &Point, rng: &mut rand::rngs::StdRng| -> (Vector, f32) {
                let displacement = { other - me };
                let dist = displacement.norm_squared();
                let dir: Vector = if dist < MIN_DIST_FOR_DIR {
                    Vector::from_distribution(&distribution, rng).normalize()
                } else {
                    displacement.normalize()
                };
                let clamped_dist = clamp(dist, 0.00001, 1e10);
                (dir, clamped_dist)
            };

        for (me, other) in (0..self.nodes.len()).tuple_combinations() {
            let me_pos = self.nodes.get(me).unwrap().p;
            let other_pos = self.nodes.get(other).unwrap().p;
            let (dir, clamped_dist) = get_displacement(&me_pos, &other_pos, &mut self.rng);
            if clamped_dist > max_dist_for_force {
                continue;
            }
            // if is_connected && dist <= DIST_FOR_LINKS + 1. && dist > DIST_FOR_LINKS - 1. {
            //     continue;
            // }
            // Repelling
            // self.nodes[other].f += dt * dir.scale(1. / clamped_dist);
            let f = dir.scale(rep_force_mult / clamped_dist.sqrt());
            self.nodes[other].f += f;
            self.nodes[me].f -= f;
        }
        for SubgLink { from, to } in &self.links {
            // Attracting
            let me_pos = self.nodes.get(from.0).unwrap().p;
            let other_pos = self.nodes.get(to.0).unwrap().p;
            // This may pick another direction for zero vectors on the first tick...
            let (dir, clamped_dist) = get_displacement(&me_pos, &other_pos, &mut self.rng);
            let f = dir.scale(clamped_dist / link_force_mult);
            self.nodes[from.0].f += f;
            self.nodes[to.0].f -= f;
        }
        let mut skipped = 0;
        for n in &mut self.nodes {
            let len = n.f.norm().min(self.params.f_max) * (dt * self.curr_damping) as f32;
            if len < self.params.f_min {
                skipped += 1;
                continue;
            }
            n.p += n.f.normalize() * len;
        }
        self.t_total += dt;
        self.curr_damping = self
            .params
            .decay_rate
            .powf((self.t_total - T_UNDAMPED).max(0.0));
        self.settled = skipped == self.nodes.len();
        self.settled
    }
}

struct GraphViewState {
    force_min: f32,
    force_max: f32,
    decay_rate: f32,
    desired_dist: f32,
    zoom: f32,
    offset: egui::Vec2,
    previous_size: egui::Vec2,
}

impl Default for GraphViewState {
    fn default() -> Self {
        let l = LayoutOptimizerParams::default();
        Self {
            force_min: l.f_min,
            force_max: l.f_max,
            decay_rate: l.decay_rate as f32,
            desired_dist: l.desired_dist,
            zoom: 1.,
            offset: egui::Vec2::default(),
            previous_size: egui::Vec2::default(),
        }
    }
}

impl GraphViewState {
    fn text_alpha(&self) -> f32 {
        ((self.zoom - 10.) / 20.).clamp(0., 1.)
    }
}

struct Filter {
    node_title: String,
    tag_state: TagFilter,

    show_connected: bool,
    max_nbrs: usize,
    filter_orphans: bool,
}

impl Filter {
    fn from_graph(graph: &Graph) -> Self {
        let tag_states = graph
            .tags
            .all_tags()
            .map(|(t, _)| (t.to_string(), TagFilterState::Include))
            .collect();
        Self {
            node_title: String::new(),
            tag_state: TagFilter {
                text_filter: String::default(),
                states: tag_states,
            },
            show_connected: true,
            max_nbrs: 20,
            filter_orphans: false,
        }
    }

    fn apply_to(&self, graph: &Graph) -> GraphLayout {
        // TODO: chainable filters with .and() and .or() or something
        // Building up the filter this way sucks
        let title = self.node_title.as_str();
        let tag_filter = self.tag_state.build_filter();
        let orphan_filter = |n: &&Node| {
            if self.filter_orphans {
                n.degree() > 0
            } else {
                true
            }
        };
        if title.is_empty() {
            if !self.tag_state.is_active() {
                return GraphLayout::new(graph.nodes().filter(orphan_filter), graph, 0);
            }
            return GraphLayout::new(
                graph
                    .nodes()
                    .filter(orphan_filter)
                    .filter(|n| tag_filter(n)),
                graph,
                0,
            );
        }
        let case_sensitive = title.chars().any(char::is_uppercase);
        let lower_title = title.to_lowercase();
        let matcher = |n: &&Node| {
            orphan_filter(n) && tag_filter(n) && {
                if case_sensitive {
                    n.title.contains(title)
                } else {
                    n.title.to_lowercase().contains(lower_title.as_str())
                }
            }
        };
        if !self.show_connected || self.max_nbrs == 0 {
            GraphLayout::new(graph.nodes().filter(matcher), graph, 0)
        } else {
            // TODO: 2-stage hilighting: direct and connected nodes
            // TODO: Should tag filtering also apply here? If we set something to excluded, it
            // should probably be hidden here too.
            GraphLayout::new(
                graph
                    .nodes()
                    .filter(matcher)
                    .flat_map(|n| graph.dfs_limited(n.id, self.max_nbrs))
                    .unique_by(|n| n.id),
                graph,
                0,
            )
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum TagFilterState {
    Include,
    Exclusive,
    Exclude,
}

impl TagFilterState {
    fn next(&self) -> Self {
        use TagFilterState::*;
        match self {
            Include => Exclusive,
            Exclusive => Exclude,
            Exclude => Include,
        }
    }
}

impl Display for TagFilterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TagFilterState::Exclude => f.write_str("x"),
            TagFilterState::Include => f.write_str(" "),
            TagFilterState::Exclusive => f.write_str("i"),
        }
    }
}

struct TagFilter {
    /// UI only, visually filter tag list
    text_filter: String,
    states: Vec<(String, TagFilterState)>,
}

impl TagFilter {
    fn is_active(&self) -> bool {
        self.states
            .iter()
            .any(|(_, state)| *state != TagFilterState::Include)
    }

    fn build_filter<'a>(&'a self) -> Box<dyn Fn(&Node) -> bool + 'a> {
        use TagFilterState::*;
        {
            let exclusive_tags: Vec<&str> = self
                .states
                .iter()
                .filter(|(_, s)| *s == Exclusive)
                .map(|(s, _)| s.as_str())
                .collect();
            if !exclusive_tags.is_empty() {
                return Box::new(move |n: &Node| -> bool {
                    !n.tags.is_empty()
                        && n.tags.iter().any(|t| exclusive_tags.contains(&t.as_str()))
                });
            }
        }
        {
            let excluded_tags: Vec<&str> = self
                .states
                .iter()
                .filter(|(_, s)| *s == Exclude)
                .map(|(s, _)| s.as_str())
                .collect();
            Box::new(move |n: &Node| -> bool {
                n.tags.is_empty() || n.tags.iter().all(|t| !excluded_tags.contains(&t.as_str()))
            })
        }
    }
}

struct FilterResponse<'a> {
    changed: bool,
    hovered_tag: Option<&'a str>,
}

fn filter_ui<'a>(ui: &mut egui::Ui, filter: &'a mut Filter) -> FilterResponse<'a> {
    let title_changed = {
        ui.label("Title must include:");
        ui.add(egui::TextEdit::singleline(&mut filter.node_title))
            .changed()
    };
    ui.separator();

    let (tag_changed, hovered_tag) = {
        let state = &mut filter.tag_state;
        ui.horizontal(|ui| {
            let tag_label = ui.label("Tags");
            ui.add(egui::TextEdit::singleline(&mut state.text_filter))
                .labelled_by(tag_label.id);
        });
        let mut changed = false;
        ui.horizontal(|ui| {
            if ui.button("r").clicked() {
                for (_, s) in &mut state.states {
                    *s = TagFilterState::Include;
                }
                changed = true;
            }
            if ui.button("x").clicked() {
                for (_, s) in &mut state.states {
                    *s = TagFilterState::Exclude;
                }
                changed = true;
            }
            if ui.button("i").clicked() {
                for (_, s) in &mut state.states {
                    *s = TagFilterState::Exclusive;
                }
                changed = true;
            }
        });
        let must_filter = !state.text_filter.is_empty();
        let filter_pat = state.text_filter.to_lowercase();
        let mut res: Option<&str> = None;
        for (name, state) in &mut state.states {
            if !must_filter || name.to_lowercase().contains(&filter_pat) {
                let r = ui.label(format!("{name} {state}"));
                if r.clicked() {
                    *state = state.next();
                    changed = true;
                }
                if r.contains_pointer() {
                    res = Some(name);
                }
            }
        }

        (changed, res)
    };
    ui.separator();
    let orphans_changed = ui
        .checkbox(&mut filter.filter_orphans, "Hide orphans")
        .changed();
    ui.separator();
    // If both of the others are false, this has no effect
    let nbrs_changed = {
        let checkbox_changed = ui
            .checkbox(&mut filter.show_connected, "include connected nodes")
            .changed();
        ui.label("Limit to");
        let depth_changed = ui
            .add(
                egui::Slider::new(&mut filter.max_nbrs, 0_usize..=20)
                    .clamping(egui::SliderClamping::Never),
            )
            .changed();
        checkbox_changed || depth_changed
    };
    FilterResponse {
        changed: orphans_changed || nbrs_changed || tag_changed || title_changed,
        hovered_tag,
    }
}

struct RoamUI {
    commands: CommandIPC,
    graph: Graph,
    layout: GraphLayout,
    history: History<NodeId>,
    view_state: GraphViewState,
    filter: Filter,
    selected: Option<NodeDetails>,
    highlighted_graph: Option<NodeId>,
    highlighted_sidebar: Option<NodeId>,
    additional_highlighted: Vec<NodeId>, // TODO: HashSet may or may not be faster, but n is small
}

const RADIUS: f32 = 1.0;

impl RoamUI {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // customization here
        let graph = load_graph();
        let filter = Filter::from_graph(&graph);
        let layout = filter.apply_to(&graph);
        let cc_clone = cc.egui_ctx.clone();
        let (commands, _) = CommandIPC::new(move || {
            cc_clone.request_repaint();
        });
        for t in [egui::Theme::Dark, egui::Theme::Light] {
            cc.egui_ctx.style_mut_of(t, |s| {
                set_theme(t, s);
            });
        }
        Self {
            commands,
            graph,
            layout,
            history: History::default(),
            view_state: GraphViewState::default(),
            filter,
            selected: None,
            highlighted_graph: None,
            highlighted_sidebar: None,
            additional_highlighted: Vec::new(),
        }
    }

    fn process_commands(&mut self, ctx: &egui::Context) {
        if let Some(cmd) = self.commands.try_pull() {
            match cmd {
                commands::Command::Quit => {
                    ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Close);
                }
                commands::Command::Echo(s) => println!("echo {s}"),
                commands::Command::Select(s) => {
                    if let Some(node) = self.graph.node_by_uuid(&s) {
                        self.select_node(node.id);
                    }
                }
            }
        }
    }

    fn apply_filter(&mut self) {
        self.layout = self.filter.apply_to(&self.graph);
        self.apply_view_state();
    }

    fn node_title_in_graph(
        &self,
        painter: &egui::Painter,
        node: &PlacedNode,
        text_color: egui::Color32,
    ) {
        painter.text(
            self.layout
                .pt_to_screen(node.p - Vector::new(0.0, 1.0) * (RADIUS + 0.5)),
            egui::Align2::CENTER_CENTER,
            &self.graph.node(node.graph_node_id).unwrap().title,
            egui::FontId::default(),
            text_color,
        );
    }

    fn render_selected(
        &self,
        ui: &mut egui::Ui
    ) -> (Option<NodeId>, Option<NodeId>, Option<String>) {
        let Some(details) = &self.selected else {
            return (None, None, None);
        };
        let mut selected_tag = None;
        let mut clicked = None;
        let mut highlighted = None;
        let node = self.graph.node(details.node).expect("node exists");
        let hl_node_col = graph_style(ui.ctx().theme()).node.hover;

        let mut render_link = |ui: &mut egui::Ui, (id, text): &(NodeId, String)| {
            let externally_highlighted =
                matches!(self.highlighted_graph, Some(hl_id) if hl_id == *id);
            ui.style_mut().visuals.widgets.hovered.fg_stroke.color = hl_node_col;
            let mut response = ui.add(egui::widgets::Label::new(text).sense(Sense::click()));
            if response.hovered() {
                highlighted = Some(*id);
            }
            if externally_highlighted {
                // TODO: adds an underline, not super nice
                response = response.highlight();
            }
            if response.clicked() {
                clicked = Some(*id);
            }
        };

        egui::Panel::right("selected")
            .exact_size(200.)
            .resizable(false)
            .show_inside(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(&node.title);
                        for alias in &node.aliases {
                            ui.label(format!("(alias {alias})"));
                        }
                        ui.separator();
                        if !node.tags.is_empty() {
                            ui.horizontal_wrapped(|inner| {
                                inner.label("Tags:");
                                for (n, t) in node.tags.iter().enumerate() {
                                    if n > 0 {
                                        inner.separator();
                                    }
                                    let r = inner.label(t);
                                    if r.contains_pointer() {
                                        selected_tag = Some(t.clone());
                                    }
                                }
                            });
                            ui.separator();
                        }
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
        (clicked, highlighted, selected_tag)
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

    fn draw_links(&self, painter: &egui::Painter) {
        let alpha = if self.selected.is_some() { 0.5 } else { 1.0 };
        let theme = graph_style(painter.ctx().theme());
        let regular_stroke = egui::Stroke::new(alpha, theme.edge);

        // All links
        for l in &self.layout.links {
            let left = self.layout.node_screen_pos(l.from);
            let right = self.layout.node_screen_pos(l.to);
            painter.line_segment([left, right], regular_stroke);
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
            let link_stroke = egui::Stroke::new(1.0, theme.out_link);
            let backlink_stroke = egui::Stroke::new(1.0, theme.backlink);
            for other_graph in self.graph.direct_links(selection.node) {
                if let Some(other_placed) = self.layout.by_real_id(other_graph.id) {
                    let left = self.layout.node_screen_pos(in_layout.layout_id);
                    let right = self.layout.node_screen_pos(other_placed.layout_id);
                    painter.line_segment([left, right], link_stroke);
                }
            }
            for other_graph in self.graph.direct_backlinks(selection.node) {
                if let Some(other_placed) = self.layout.by_real_id(other_graph.id) {
                    let left = self.layout.node_screen_pos(other_placed.layout_id);
                    let right = self.layout.node_screen_pos(in_layout.layout_id);
                    painter.line_segment([left, right], backlink_stroke);
                }
            }
        }
    }

    fn zoom_by(&mut self, delta_y: f32, pointer_pos: Option<egui::Pos2>) {
        let world_before_zoom = pointer_pos.map(|pos| {
            self.layout
                .to_screen
                .inverse_transform_point(&Point::new(pos.x, pos.y))
        });
        self.view_state.zoom = (self.view_state.zoom
            + (self.view_state.zoom / 400.).clamp(0., 0.1) * delta_y)
            .clamp(0.1, 400.0);
        self.layout.to_screen.set_scaling(self.view_state.zoom);
        let world_after = world_before_zoom.map(|pos| {
            self.layout
                .to_screen
                .transform_point(&Point::new(pos.x, pos.y))
        });
        if let (Some(before), Some(after)) = (pointer_pos, world_after) {
            let diff = egui::Vec2::new(before.x - after.x, before.y - after.y);
            self.pan_by(diff);
        }
    }

    fn pan_by(&mut self, delta: egui::Vec2) {
        self.view_state.offset += delta;
        self.layout.to_screen.isometry.translation.x = self.view_state.offset.x;
        self.layout.to_screen.isometry.translation.y = self.view_state.offset.y;
    }

    fn apply_view_state(&mut self) {
        self.layout.to_screen.set_scaling(self.view_state.zoom);
        self.layout.to_screen.isometry.translation.x = self.view_state.offset.x;
        self.layout.to_screen.isometry.translation.y = self.view_state.offset.y;
    }

    fn is_highlighted(&self, id: NodeId) -> bool {
        matches!(&self.highlighted_graph, Some(graph_highlighted) if *graph_highlighted == id)
            || matches!(&self.highlighted_sidebar, Some(graph_highlighted) if *graph_highlighted == id)
    }

    fn render_graph(&mut self, ui: &mut egui::Ui) {
        let node_colors = graph_style(ui.ctx().theme()).node;
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let mut clicked = false;
            if ui.ui_contains_pointer() {
                ui.input(|i| {
                    clicked = i.pointer.primary_clicked();
                    // TODO: zoom to mouse pos
                    if i.smooth_scroll_delta.y != 0. {
                        self.zoom_by(i.smooth_scroll_delta.y, i.pointer.latest_pos());
                    }
                    if i.pointer.primary_down() && i.pointer.is_decidedly_dragging() {
                        self.pan_by(i.pointer.delta());
                    }
                });
            }

            let size = ui.input(|i| i.viewport().inner_rect).map_or_else(||ui.min_size(), |r| r.max - r.min);
            if size != self.view_state.previous_size {
                let diff = (size - self.view_state.previous_size) * 0.5;
                self.pan_by(diff);
                self.view_state.previous_size = size;
            }
            let painter = ui.painter();
            self.layout.set_params(LayoutOptimizerParams{
                f_min: self.view_state.force_min,
                f_max: self.view_state.force_max,
                decay_rate: f64::from(self.view_state.decay_rate),
                desired_dist: self.view_state.desired_dist
            });
            let dt = f64::from(ui.input(|i|i.stable_dt));
            let settled = self.layout.double_tick(dt);
            let mut hovered_node = None;
            let radius_screen = self.layout.len_to_screen(RADIUS);

            self.draw_links(painter);

            for n in &self.layout.nodes {
                let pos = self.layout.pt_to_screen(n.p);
                let mouse_over = ui
                    .pointer_latest_pos()
                    .is_some_and(|p| (p - pos).length_sq() <= radius_screen * radius_screen);
                let (color, size) =
                    if mouse_over || self.is_highlighted(n.graph_node_id) {
                        (node_colors.hover, radius_screen * 1.1)
                    } else if matches!(&self.selected.as_ref().map(|n|n.node), Some(id) if *id == n.graph_node_id) {
                        (node_colors.selected, radius_screen)
                    } else if self.additional_highlighted.contains(&n.graph_node_id) {
                        (node_colors.highlight, radius_screen * 1.1)
                    } else {
                        (node_colors.color, radius_screen)
                    };
                painter.circle_filled(
                    pos,
                    size,
                    color
                );
                if mouse_over {
                    hovered_node = Some(n.graph_node_id);
                }
            }
            self.highlighted_graph = hovered_node;
            if clicked && hovered_node.map(|n| self.select_node(n)).is_none() {
                self.deselect_node();
            }
            let text_alpha = self.view_state.text_alpha();
            let text_color = if ui.theme() == egui::Theme::Dark {
                GRUVBOX.light1
            } else {
                GRUVBOX.dark0
            }.gamma_multiply(text_alpha);
            let selected_text = GRUVBOX.neutral_orange;
            for n in &self.layout.nodes {
                if matches!(&self.selected.as_ref().map(|n|n.node), Some(id) if *id == n.graph_node_id) {
                    self.node_title_in_graph(painter, n, selected_text);
                } else if text_alpha > 0. {
                    self.node_title_in_graph(painter, n, text_color);
                }
            }
            if ui.ui_contains_pointer() && let Some(id) = hovered_node {
                let graph_node = self.graph.node(id).unwrap();
                egui::Tooltip::always_open(
                    ui.ctx().clone(),
                    painter.layer_id(),
                    egui::Id::new("title"),
                    egui::PopupAnchor::Pointer).show(
                    |ui| {
                        let label =
                        egui::Label::new(&graph_node.title).wrap_mode(egui::TextWrapMode::Extend);
                        ui.add(label);
                    },
                );
            }
            if !settled {
                ui.request_repaint();
            }
        });
    }

    fn graph_settings(&mut self, ui: &mut egui::Ui) {
        ui.label("desired_dist");
        ui.add(egui::Slider::new(
            &mut self.view_state.desired_dist,
            0_f32..=100.,
        ));
        ui.label("f_min");
        ui.add(egui::Slider::new(
            &mut self.view_state.force_min,
            0_f32..=1.,
        ));
        ui.label("f_max");
        ui.add(egui::Slider::new(
            &mut self.view_state.force_max,
            1_f32..=1000.,
        ));
        ui.label("decay");
        ui.add(egui::Slider::new(
            &mut self.view_state.decay_rate,
            0_f32..=1.,
        ));
        if ui.button("Reset").clicked() {
            self.layout.reset_layout();
        }
        ui.separator();
        ui.label(format!("damping: {:.4}", self.layout.curr_damping));
        ui.label(format!("t: {:.3}s", self.layout.t_total));
        ui.label(format!("settled: {}", self.layout.settled));
    }
}

impl eframe::App for RoamUI {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.process_commands(ui);
        let mut filter_changed = false;
        egui::Panel::left("Filters")
            .resizable(false)
            .show_inside(ui, |ui| {
                ui.label("Filters");
                ui.separator();
                let filter_res = filter_ui(ui, &mut self.filter);
                if filter_res.changed {
                    filter_changed = true;
                }
                self.additional_highlighted.clear();
                if let Some(hovered) = filter_res.hovered_tag {
                    self.additional_highlighted =
                        self.graph.tags.node_ids_for(hovered).copied().collect();
                }
                ui.separator();
                ui.label(format!(
                    "Showing {} / {}",
                    self.layout.nodes.len(),
                    self.graph.len()
                ));
                ui.separator();
                // TODO: ugly animation
                ui.collapsing("Layout settings", |ui| self.graph_settings(ui));

                egui::Panel::bottom("settings")
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                            egui::widgets::global_theme_preference_switch(ui);
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::LEFT), |ui| {
                                ui.label("Theme");
                            });
                        });
                    });
            });
        let (next_sel, next_hl, sel_tag) = self.render_selected(ui);
        if let Some(next_selection) = next_sel {
            self.select_node(next_selection);
        }
        if let Some(sel_tag) = sel_tag {
            self.additional_highlighted = self
                .graph
                .tags
                .node_ids_for(sel_tag.as_str())
                .copied()
                .collect();
        }

        if filter_changed {
            self.apply_filter();
        }
        self.highlighted_sidebar = next_hl;

        self.render_graph(ui);
        ui.input(|i| self.handle_global_shortcuts(i));
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
