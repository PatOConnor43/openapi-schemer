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
    // TODO refactor this query string into the `bindings` module.
    // This query_string works by matching all block_mapping_pairs that equal "paths".
    // It captures by using the `@` after the node. So the first capture is `@key` which
    // is then used to do an `eq?` check. Then we drill down in the value until we find
    // a `flow_node` and capture that with `@inner_key`. All of the matching `flow_node`
    // should be paths. This module is actually supposed to be printing operations but
    // this was an easy first step.
    //
    let query_string = r#"(block_mapping_pair key: ((flow_node) @key (eq? @key "paths")) value: (block_node (block_mapping (block_mapping_pair (flow_node) @inner_key))))"#;
    let query = Query::new(language, query_string).expect("Could not construct query");
    let mut qc = QueryCursor::new();
    let provider = contents.as_bytes();

    for qm in qc.matches(&query, tree.root_node(), provider) {
        if let Some(cap) = qm.captures.get(1) {
            if let Ok(route) = cap.node.utf8_text(provider) {
                println!("{}", route);
            }
        }
    }
}
