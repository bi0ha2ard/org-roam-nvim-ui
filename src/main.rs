use itertools::Itertools;
use nalgebra::{Point2, Vector2};
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
        self.links.iter().find(|l| (l.from == from && l.to == to) || (l.from == to && l.to == from)).is_some()
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
    rng: rand::rngs::StdRng,
}

impl GraphLayout {
    fn new(nodes: &[Node], seed: u64) -> GraphLayout {
        GraphLayout {
            nodes: nodes
                .iter()
                .map(|n| PlacedNode {
                    p: Point::origin(),
                    f: Vector::zeros(),
                    id: n.id,
                })
                .collect(),
            rng: rand::rngs::StdRng::seed_from_u64(seed)
        }
    }

    fn tick(&mut self, graph: &Graph, dt: f32) {
        const MIN_DIST_FOR_DIR: f32 = 1e-6;
        const DIST_FOR_LINKS: f32 = 1.0;
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
            // Repelling
            self.nodes[other].f += dt * dir.scale(1. / dist);
            self.nodes[me].f += dt * dir.scale(-1. / dist);
            // Attracting
            if dist > DIST_FOR_LINKS && graph.is_connected(me, other) {
                self.nodes[other].f -= dt * dir.scale(1. / dist);
                self.nodes[me].f -= dt * dir.scale(-1. / dist);
            }
        }
        for n in &mut self.nodes {
            // TODO: clamp forces
            n.p += n.f;
        }
    }
}

fn main() {
    const DB_FNAME: &str = "db_pretty.json";
    const ORG_ROAM_SHARE_DIR: &str = ".local/share/nvim/org-roam.nvim";
    let roam_share_loc = std::path::Path::new(&std::env::var_os("HOME").expect("home"))
        .join(ORG_ROAM_SHARE_DIR)
        .join(DB_FNAME);
    let file = std::fs::File::open(roam_share_loc).expect("Open");
    let db: Database = serde_json::from_reader(std::io::BufReader::new(file)).expect("Parse");
    let g = Graph::from(db);

    print!("{}", g.dot());
}
