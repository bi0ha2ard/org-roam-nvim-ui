use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

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

#[derive(Default, Eq, PartialEq, PartialOrd, Ord, Copy, Clone, Hash)]
pub struct NodeId(usize);

#[derive(Deserialize)]
pub struct Node {
    /// Our internal id, just counted
    #[serde(skip)]
    pub id: NodeId,

    /// index range into graph.links
    #[serde(skip)]
    links: Range<usize>,

    /// index range into graph.links
    #[serde(skip)]
    backlinks: Range<usize>,

    pub tags: Vec<String>,
    pub aliases: Vec<String>,
    #[serde(rename(deserialize = "id"))]
    pub uuid: String,
    pub level: i32,
    pub title: String,
    pub mtime: u64,
}

#[derive(PartialEq)]
pub struct Link {
    pub from: NodeId,
    pub to: NodeId,
}

pub struct Graph {
    nodes: Vec<Node>,
    links: Vec<Link>,     // from -> to, sorted by from
    backlinks: Vec<Link>, // from -> to, sorted by to
}

// TODO: reference the &str of the graph object
pub struct NodeDetails {
    pub node: NodeId,
    pub links: Vec<(NodeId, String)>,     // target id, target title
    pub backlinks: Vec<(NodeId, String)>, // source id, source title
}

#[derive(PartialEq)]
pub enum DfsDirection {
    Out,
    In,
    Both,
}

impl DfsDirection {
    fn allows_out(&self) -> bool {
        use DfsDirection::*;
        *self == Out || *self == Both
    }

    fn allows_in(&self) -> bool {
        use DfsDirection::*;
        *self == In || *self == Both
    }
}

pub struct DfsIterator<'a> {
    graph: &'a Graph,
    to_visit: Vec<NodeId>,
    visited: HashSet<NodeId>,
    dir: DfsDirection,
}

impl<'a> DfsIterator<'a> {
    fn new(graph: &'a Graph, from: NodeId, dir: DfsDirection) -> DfsIterator<'a> {
        DfsIterator {
            graph,
            to_visit: vec![from],
            visited: HashSet::new(),
            dir,
        }
    }
}

impl<'a> Iterator for DfsIterator<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.to_visit.pop() {
            // TODO: allow limiting depth
            self.visited.insert(next);
            if self.dir.allows_out() {
                for nbrs in self.graph.direct_links(next) {
                    if !self.visited.contains(&nbrs.id) {
                        self.to_visit.push(nbrs.id);
                    }
                }
            }
            if self.dir.allows_in() {
                for nbrs in self.graph.direct_backlinks(next) {
                    if !self.visited.contains(&nbrs.id) {
                        self.to_visit.push(nbrs.id);
                    }
                }
            }
            return Some(self.graph.node(next).expect("Node should be in graph"));
        }
        None
    }
}

impl Graph {
    fn from(db: Database) -> Graph {
        let mut nodes: Vec<Node> = db.nodes.into_values().collect();
        for (id, n) in nodes.iter_mut().enumerate() {
            n.id = NodeId(id);
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
                            from: NodeId(*tmp.get(k.as_str()).expect("from")),
                            to: NodeId(*tmp.get(to.as_str()).expect("to")),
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
                    for backlink_source in l.keys() {
                        backlinks.push(Link {
                            from: NodeId(*tmp.get(backlink_source.as_str()).expect("from")),
                            to: NodeId(*tmp.get(k.as_str()).expect("to")),
                        });
                    }
                }
            }
        }
        links.sort_by_key(|l| l.from.0);
        backlinks.sort_by_key(|l| l.to.0);
        {
            let mut current_idx = NodeId(0);
            let mut last_start = 0;
            for (idx, n) in links.iter().enumerate() {
                if n.from != current_idx {
                    nodes[current_idx.0].links.start = last_start;
                    nodes[current_idx.0].links.end = idx;
                    last_start = idx;
                    current_idx = n.from;
                }
            }
            nodes[current_idx.0].links.start = last_start;
            nodes[current_idx.0].links.end = links.len();
        }
        {
            let mut current_idx = NodeId(0);
            let mut last_start = 0;
            for (idx, n) in backlinks.iter().enumerate() {
                if n.to != current_idx {
                    nodes[current_idx.0].backlinks.start = last_start;
                    nodes[current_idx.0].backlinks.end = idx;
                    last_start = idx;
                    current_idx = n.to;
                }
            }
            nodes[current_idx.0].backlinks.start = last_start;
            nodes[current_idx.0].backlinks.end = links.len();
        }

        Graph {
            nodes,
            links,
            backlinks,
        }
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter()
    }

    pub fn links(&self) -> impl Iterator<Item = &Link> {
        self.links.iter()
    }

    pub fn backlinks(&self) -> impl Iterator<Item = &Link> {
        self.backlinks.iter()
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0)
    }

    pub fn bfs(&self, id: NodeId) -> impl Iterator<Item = &Node> {
        DfsIterator::new(self, id, DfsDirection::Both)
    }

    pub fn direct_links(&self, id: NodeId) -> impl Iterator<Item = &Node> {
        self.links[self.nodes[id.0].links.clone()]
            .iter()
            .map(|l| self.node(l.to).unwrap())
    }

    pub fn direct_backlinks(&self, id: NodeId) -> impl Iterator<Item = &Node> {
        self.backlinks[self.nodes[id.0].backlinks.clone()]
            .iter()
            .map(|l| self.node(l.from).unwrap())
    }

    fn is_connected(&self, from: NodeId, to: NodeId) -> bool {
        self.links
            .iter()
            .any(|l| (l.from == from && l.to == to) || (l.from == to && l.to == from))
    }

    pub fn node_details(&self, node: NodeId) -> NodeDetails {
        let to_tuple = |l: &Node| (l.id, l.title.clone());
        NodeDetails {
            node,
            links: self.direct_links(node).map(to_tuple).collect(),
            backlinks: self.direct_backlinks(node).map(to_tuple).collect(),
        }
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
                self.node(l.from).unwrap().title,
                self.node(l.to).unwrap().title
            ));
        }
        for l in &self.backlinks {
            res.push_str(&format!(
                "\"{}\" -> \"{}\" [color=red];\n",
                self.node(l.from).unwrap().title,
                self.node(l.to).unwrap().title
            ));
        }
        res.push('}');
        res
    }
}

pub fn load_graph() -> Graph {
    const DB_FNAME: &str = "db_pretty.json";
    const ORG_ROAM_SHARE_DIR: &str = ".local/share/nvim/org-roam.nvim";
    let roam_share_loc = std::path::Path::new(&std::env::var_os("HOME").expect("home"))
        .join(ORG_ROAM_SHARE_DIR)
        .join(DB_FNAME);
    let file = std::fs::File::open(roam_share_loc).expect("Open");
    let db: Database = serde_json::from_reader(std::io::BufReader::new(file)).expect("Parse");
    Graph::from(db)
}

