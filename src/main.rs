use eframe::egui;
use egui::Pos2;
use itertools::Itertools;
use nalgebra::{Point2, Similarity2, Translation2, Vector2, clamp};
use rand::SeedableRng;
use std::collections::HashMap;

use serde::{Deserialize, de::IgnoredAny};

#[derive(Deserialize)]
#[serde(untagged)]
enum DBLink {
    Links(HashMap<String, bool>),
    Empty(IgnoredAny),
}

#[derive(Deserialize)]
struct Database {
    nodes: HashMap<String, Node>,
    outbound: HashMap<String, DBLink>,
    inbound: HashMap<String, DBLink>,
}

#[derive(Deserialize)]
struct Node {
    #[serde(skip)]
    id: usize,
    tags: Vec<String>,
    aliases: Vec<String>,
    #[serde(rename(deserialize = "id"))]
    uuid: String,
    level: i32,
    title: String,
    mtime: u64,
}

#[derive(PartialEq)]
struct Link {
    from: usize,
    to: usize,
}

struct Graph {
    nodes: Vec<Node>,
    links: Vec<Link>,     // from -> to, sorted by from
    backlinks: Vec<Link>, // from -> to, sorted by to
}

impl Graph {
    fn from(db: Database) -> Graph {
        let mut nodes: Vec<Node> = db.nodes.into_values().collect();
        for (id, n) in nodes.iter_mut().enumerate() {
            n.id = id;
        }
        let mut tmp = HashMap::<&str, usize>::new();
        for (id, n) in nodes.iter().enumerate() {
            tmp.insert(n.uuid.as_str(), id);
        }
        let mut links = Vec::new();
        for (k, v) in db.outbound {
            match v {
                DBLink::Empty(_) => {}
                DBLink::Links(l) => {
                    for to in l.keys() {
                        links.push(Link {
                            from: *tmp.get(k.as_str()).expect("from"),
                            to: *tmp.get(to.as_str()).expect("to"),
                        });
                    }
                }
            }
        }
        let mut backlinks = Vec::new();
        for (k, v) in db.inbound {
            match v {
                DBLink::Empty(_) => {}
                DBLink::Links(l) => {
                    for to in l.keys() {
                        backlinks.push(Link {
                            from: *tmp.get(k.as_str()).expect("from"),
                            to: *tmp.get(to.as_str()).expect("to"),
                        });
                    }
                }
            }
        }
        links.sort_by_key(|l| l.from);
        backlinks.sort_by_key(|l| l.to);
        Graph {
            nodes,
            links,
            backlinks,
        }
    }

    fn links(&self, id: usize) -> impl Iterator<Item = &Node> {
        self.links
            .iter()
            .skip_while(move |l| id != l.from)
            .take_while(move |l| id == l.from)
            .map(|l| self.nodes.get(l.to).unwrap())
    }

    fn backlinks(&self, id: usize) -> impl Iterator<Item = &Node> {
        self.backlinks
            .iter()
            .skip_while(move |l| id != l.to)
            .take_while(move |l| id == l.to)
            .map(|l| self.nodes.get(l.to).unwrap())
    }

    fn is_connected(&self, from: usize, to: usize) -> bool {
        self.links
            .iter()
            .any(|l| (l.from == from && l.to == to) || (l.from == to && l.to == from))
    }

    fn dot(&self) -> String {
        let mut res = String::new();
        res.push_str("digraph {");
        for n in &self.nodes {
            res.push_str(&format!("\"{}\";\n", n.title));
        }
        for l in &self.links {
            res.push_str(&format!(
                "\"{}\" -> \"{}\" [color=blue];\n",
                self.nodes.get(l.from).unwrap().title,
                self.nodes.get(l.to).unwrap().title
            ));
        }
        for l in &self.backlinks {
            res.push_str(&format!(
                "\"{}\" -> \"{}\" [color=red];\n",
                self.nodes.get(l.to).unwrap().title,
                self.nodes.get(l.from).unwrap().title
            ));
        }
        res.push('}');
        res
    }
}

type Point = Point2<f32>;
type Vector = Vector2<f32>;

struct PlacedNode {
    p: Point,  // Location
    f: Vector, // yet to be applied force
    id: usize, // keep track of id in case we're filtered down later
}

struct GraphLayout {
    // Current positions
    nodes: Vec<PlacedNode>,
    links: Vec<Link>,
    backlinks: Vec<Link>,
    rng: rand::rngs::StdRng,
    to_screen: Similarity2<f32>,
}

impl GraphLayout {
    // New layout for nodes
    fn new(nodes: &[Node], graph: &Graph, seed: u64) -> GraphLayout {
        let nodes: Vec<PlacedNode> = nodes
            .iter()
            .map(|n| PlacedNode {
                p: Point::origin(),
                f: Vector::zeros(),
                id: n.id,
            })
            .collect();
        // TODO: factor out slicing of the graph?
        let mut links = Vec::new();
        let mut backlinks = Vec::new();
        for (me, other) in nodes.iter().enumerate().tuple_combinations() {
            if graph.links.contains(&Link {
                from: me.1.id,
                to: other.1.id,
            }) {
                links.push(Link {
                    from: me.0,
                    to: other.0,
                });
            }
            if graph.backlinks.contains(&Link {
                from: other.1.id,
                to: me.1.id,
            }) {
                backlinks.push(Link {
                    from: other.0,
                    to: me.0,
                });
            }
        }
        links.sort_by_key(|l| l.from);
        backlinks.sort_by_key(|l| l.to);

        GraphLayout {
            nodes,
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

    fn node_screen_pos(&self, node: usize) -> egui::Pos2 {
        self.pt_to_screen(self.nodes.get(node).unwrap().p)
    }

    fn is_connected(&self, from: usize, to: usize) -> bool {
        self.links
            .iter()
            .any(|l| (l.from == from && l.to == to) || (l.from == to && l.to == from))
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
            let is_connected = self.is_connected(me, other);
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

fn load_graph() -> Graph {
    const DB_FNAME: &str = "db_pretty.json";
    const ORG_ROAM_SHARE_DIR: &str = ".local/share/nvim/org-roam.nvim";
    let roam_share_loc = std::path::Path::new(&std::env::var_os("HOME").expect("home"))
        .join(ORG_ROAM_SHARE_DIR)
        .join(DB_FNAME);
    let file = std::fs::File::open(roam_share_loc).expect("Open");
    let db: Database = serde_json::from_reader(std::io::BufReader::new(file)).expect("Parse");
    Graph::from(db)
}

struct State {
    dt: f32,
    force_min: f32,
    force_max: f32,
    zoom: f32,
    offset: egui::Vec2,
}

impl Default for State {
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

impl State {
    fn text_alpha(&self) -> f32 {
        ((self.zoom - 10.) / 20.).clamp(0., 1.)
    }
}

struct RoamUI {
    graph: Graph,
    layout: GraphLayout,
    state: State,
}

impl RoamUI {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // customization here
        let graph = load_graph();
        let layout = GraphLayout::new(&graph.nodes, &graph, 0);
        Self {
            graph,
            layout,
            state: State::default(),
        }
    }
}

impl eframe::App for RoamUI {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Q)) {
                ctx.send_viewport_cmd(egui::viewport::ViewportCommand::Close);
            }
            ui.add(egui::Slider::new(&mut self.state.dt, 0.001_f32..=5.));
            ui.add(egui::Slider::new(&mut self.state.force_max, 1_f32..=10.));
            ui.add(egui::Slider::new(&mut self.state.force_min, 0_f32..=1.));
            if ui.button("Reset").clicked() {
                for n in &mut self.layout.nodes {
                    n.p = Point::origin();
                }
            }
            // TODO: Filter to only the painter this only on the background?
            ui.input(|i| {
                if i.smooth_scroll_delta.y != 0. {
                    self.state.zoom = (self.state.zoom
                        + (self.state.zoom / 400.).clamp(0., 0.1) * i.smooth_scroll_delta.y)
                        .clamp(0.1, 400.0);
                }
                if i.pointer.is_decidedly_dragging() {
                    self.state.offset += i.pointer.delta()
                }
            });
            // TODO: zoom to mouse pos
            self.layout.to_screen.set_scaling(self.state.zoom);
            self.layout.to_screen.isometry.translation.x = self.state.offset.x;
            self.layout.to_screen.isometry.translation.y = self.state.offset.y;

            let offs = ui.min_size() / 2.0;
            let painter = ui.painter();
            let settled =
                self.layout
                    .tick(self.state.dt, self.state.force_min, self.state.force_max);
            let mut selected_node = None;
            const RADIUS: f32 = 1.0;
            let radius_screen = self.layout.len_to_screen(RADIUS);
            let conn_stroke = egui::Stroke::new(1.0, egui::Color32::RED);

            for l in &self.layout.links {
                let left = self.layout.node_screen_pos(l.from);
                let right = self.layout.node_screen_pos(l.to);
                painter.line_segment([left + offs, right + offs], conn_stroke);
            }
            for n in &self.layout.nodes {
                let pos = self.layout.pt_to_screen(n.p) + offs;
                let selected = ctx
                    .pointer_latest_pos()
                    .map(|p| (p - pos).length_sq() <= radius_screen * radius_screen)
                    .unwrap_or(false);
                painter.circle_filled(
                    pos,
                    radius_screen,
                    if selected {
                        egui::Color32::BLUE
                    } else {
                        egui::Color32::RED
                    },
                );
                if selected {
                    selected_node = Some(n.id);
                }
            }
            let text_alpha = self.state.text_alpha();
            if text_alpha > 0. {
                let text_color =
                    egui::Color32::from_rgba_unmultiplied(128, 128, 128, (text_alpha * 255.) as u8);
                for n in &self.layout.nodes {
                    painter.text(
                        self.layout
                            .pt_to_screen(n.p - Vector::new(0.0, 1.0) * (RADIUS + 0.5))
                            + offs,
                        egui::Align2::CENTER_CENTER,
                        &self.graph.nodes.get(n.id).unwrap().title,
                        egui::FontId::default(),
                        text_color,
                    );
                }
            }
            if let Some(id) = selected_node {
                let n = self.graph.nodes.get(id).unwrap();
                egui::show_tooltip_at_pointer(
                    ctx,
                    painter.layer_id(),
                    egui::Id::new("title"),
                    |ui| {
                        let label =
                            egui::Label::new(&n.title).wrap_mode(egui::TextWrapMode::Extend);
                        ui.add(label);
                    },
                );
            }
            if !settled {
                ctx.request_repaint();
            }
        });
    }
}

fn main() {
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "My egui App",
        native_options,
        Box::new(|cc| Ok(Box::new(RoamUI::new(cc)))),
    );
}
