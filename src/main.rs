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
    links: Vec<Link>,
    backlinks: Vec<Link>,
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
        backlinks.sort_by_key(|l| l.from);
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
            .skip_while(move |l| id != l.from)
            .take_while(move |l| id == l.from)
            .map(|l| self.nodes.get(l.to).unwrap())
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

    for n in &g.nodes {
        println!("{:>4}: {} ({})", n.id, n.title, n.uuid);
        for l in g.links(n.id) {
            println!("        > {}", l.title);
        }
        for l in g.backlinks(n.id) {
            println!("        < {}", l.title);
        }
    }
}
