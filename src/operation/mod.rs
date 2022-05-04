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

    // This query is an amalgamation of all the different supported http verbs.
    // First we find a key with the text that matches an http verb (get/post/put...),
    // then we look for a child `flow_node` that has the key `operationId`. If we find
    // one we'll try to match the value of that node with should be the name of the
    // operation.
    let query_string = r#"
        (block_mapping_pair
           key: ((flow_node) @delete (eq? @delete "delete"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: (flow_node) @deletevalue (eq? @deletevalue "operationId")
                       value: (flow_node) @deletevalue
                       ))))
        (block_mapping_pair
           key: ((flow_node) @get (eq? @get "get"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: ((flow_node) @operationId (eq? @operationId "operationId"))
                       value: (flow_node) @getvalue
                       ))))
        (block_mapping_pair
           key: ((flow_node) @head (eq? @head "head"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: ((flow_node) @operationId (eq? @operationId "operationId"))
                       value: (flow_node) @headvalue
                       ))))
        (block_mapping_pair
           key: ((flow_node) @options (eq? @options "options"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: ((flow_node) @operationId (eq? @operationId "operationId"))
                       value: (flow_node) @optionsvalue
                       ))))
        (block_mapping_pair
           key: ((flow_node) @patch (eq? @patch "patch"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: ((flow_node) @operationId (eq? @operationId "operationId"))
                       value: (flow_node) @patchvalue
                       ))))
        (block_mapping_pair
           key: ((flow_node) @post (eq? @post "post"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: ((flow_node) @operationId (eq? @operationId "operationId"))
                       value: (flow_node) @postvalue
                       ))))
        (block_mapping_pair
           key: ((flow_node) @put (eq? @put "put"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: ((flow_node) @operationId (eq? @operationId "operationId"))
                       value: (flow_node) @putvalue
                       ))))
        (block_mapping_pair
           key: ((flow_node) @connect (eq? @connect "connect"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: ((flow_node) @operationId (eq? @operationId "operationId"))
                       value: (flow_node) @connectvalue
                       ))))
        (block_mapping_pair
           key: ((flow_node) @trace (eq? @trace "trace"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: ((flow_node) @operationId (eq? @operationId "operationId"))
                       value: (flow_node) @tracevalue
                       ))))
        "#;
    let query = Query::new(language, query_string).expect("Could not construct query");
    let mut qc = QueryCursor::new();
    let provider = contents.as_bytes();

    for qm in qc.matches(&query, tree.root_node(), provider) {
        if let Some(cap) = qm.captures.get(2) {
            if let Ok(route) = cap.node.utf8_text(provider) {
                println!("{}", route);
            }
        }
    }
}
