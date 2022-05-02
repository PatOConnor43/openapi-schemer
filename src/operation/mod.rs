use std::{fs::File, io::Read, path::PathBuf};

use tree_sitter::{Parser, Query, QueryCursor};

use crate::bindings;

pub fn list(path: PathBuf) {
    let mut file = File::open(path).expect("Unable to open the file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Unable to read the file");
    let language = bindings::language();
    let mut parser = Parser::new();
    parser.set_language(language).unwrap();
    let tree = parser.parse(contents.clone(), None).unwrap();
    let query_string = r#"(block_mapping_pair key: ((flow_node) @key (eq? @key "paths")) value: (block_node (block_mapping (block_mapping_pair (flow_node) @inner_key))))"#;
    let query = Query::new(language, query_string).expect("Could not construct query");
    let mut qc = QueryCursor::new();
    let provider = contents.as_bytes();

    for qm in qc.matches(&query, tree.root_node(), provider) {
        if let Some(cap) = qm.captures.get(1) {
            if cap.node.kind() == "flow_node" {
                if let Ok(route) = cap.node.utf8_text(provider) {
                    println!("{}", route);
                }
            }
        }
    }
}
