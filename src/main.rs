use std::collections::HashMap;

use serde::{Deserialize, de::IgnoredAny};

#[derive(Deserialize)]
#[serde(untagged)]
enum Link {
    Links(HashMap<String, bool>),
    Empty(IgnoredAny),
}

#[derive(Deserialize)]
struct Database {
    nodes: HashMap<String, Node>,
    outbound: HashMap<String, Link>,
    inbound: HashMap<String, Link>,
}

#[derive(Deserialize)]
struct Node {
    tags: Vec<String>,
    aliases: Vec<String>,
    id: String,
    level: i32,
    title: String,
    mtime: u64,
}

fn main() {
    const DB_FNAME: &str = "db_pretty.json";
    const ORG_ROAM_SHARE_DIR: &str = ".local/share/nvim/org-roam.nvim";
    let roam_share_loc = std::path::Path::new(&std::env::var_os("HOME").expect("home"))
        .join(ORG_ROAM_SHARE_DIR)
        .join(DB_FNAME);
    let file = std::fs::File::open(roam_share_loc).expect("Open");
    let db: Database = serde_json::from_reader(std::io::BufReader::new(file)).expect("Parse");
    for n in db.nodes.values() {
        println!("{}: {}", n.title, n.id);
    }

    // TODO: put directly into multimap
    // TODO: parse UUIDs to avoid String?
    for (k, v) in &db.inbound {
        match v {
            Link::Empty(_) => {}
            Link::Links(links) => {
                for t in links.keys() {
                    println!("{} <- {}", k, t);
                }
            }
        }
    }
    for (k, v) in &db.outbound {
        match v {
            Link::Empty(_) => {}
            Link::Links(links) => {
                for t in links.keys() {
                    println!("{} -> {}", k, t);
                }
            }
        }
    }

    println!("{}, {}, {}", db.nodes.keys().count(), db.inbound.keys().count(), db.outbound.keys().count());
}
