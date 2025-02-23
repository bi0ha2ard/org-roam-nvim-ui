use std::collections::HashMap;

use serde::{de::IgnoredAny, Deserialize};

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
    mtime: u64
}

fn main() {
    let file = std::fs::File::open("/home/felix/.local/share/nvim/org-roam.nvim/db_pretty.json").expect("Open");
    let db: Database = serde_json::from_reader(std::io::BufReader::new(file)).expect("Parse");
    for n in db.nodes.values() {
        println!("{}: {}", n.title, n.id);
    }
    for (k, v) in db.inbound {
        match v {
            Link::Empty(_) => {},
            Link::Links(links) => {
                for t in links.keys() {
                    println!("{} <- {}", k, t);
                }
            }
        }
    }
    for (k, v) in db.outbound {
        match v {
            Link::Empty(_) => {},
            Link::Links(links) => {
                for t in links.keys() {
                    println!("{} -> {}", k, t);
                }
            }
        }
    }
}
