//! This crate provides yaml language support for the [tree-sitter][] parsing library.
//!
//! Typically, you will use the [language][language func] function to add this language to a
//! tree-sitter [Parser][], and then use the parser to parse some code:
//!
//! ```
//! let code = "";
//! let mut parser = tree_sitter::Parser::new();
//! parser.set_language(tree_sitter_yaml::language()).expect("Error loading yaml grammar");
//! let tree = parser.parse(code, None).unwrap();
//! ```
//!
//! [Language]: https://docs.rs/tree-sitter/*/tree_sitter/struct.Language.html
//! [language func]: fn.language.html
//! [Parser]: https://docs.rs/tree-sitter/*/tree_sitter/struct.Parser.html
//! [tree-sitter]: https://tree-sitter.github.io/

pub mod path;
pub mod schema;
use std::{collections::HashMap, path::PathBuf};

use tree_sitter::{Language, Node, Parser, Query, QueryCursor, TreeCursor};

extern "C" {
    fn tree_sitter_yaml() -> Language;
}

/// Get the tree-sitter [Language][] for this grammar.
///
/// [Language]: https://docs.rs/tree-sitter/*/tree_sitter/struct.Language.html
pub fn language() -> Language {
    unsafe { tree_sitter_yaml() }
}

pub trait ContentProvider {
    fn get(&self, path: PathBuf) -> String;
}

struct ContentProviderMap {
    contents: HashMap<PathBuf, String>,
}
impl ContentProvider for ContentProviderMap {
    fn get(&self, path: PathBuf) -> String {
        return match self.contents.get(&path) {
            Some(path) => path.to_string(),
            None => "".to_string(),
        };
    }
}
impl Into<ContentProviderMap> for HashMap<PathBuf, String> {
    fn into(self) -> ContentProviderMap {
        ContentProviderMap { contents: self }
    }
}

#[derive(Clone, Debug)]
pub struct OperationNode {
    pub text: String,
}

pub trait OperationParser {
    fn get_operation_nodes(&self) -> Vec<OperationNode>;
}

pub struct TreeSitterOperationParser<T: ContentProvider> {
    provider: Box<T>,
}
impl<T> TreeSitterOperationParser<T>
where
    T: ContentProvider,
{
    pub fn new(provider: T) -> TreeSitterOperationParser<T> {
        TreeSitterOperationParser {
            provider: Box::new(provider),
        }
    }
}
impl<T: ContentProvider> OperationParser for TreeSitterOperationParser<T> {
    fn get_operation_nodes(&self) -> Vec<OperationNode> {
        let mut parser = Parser::new();
        let language = language();
        parser.set_language(language).unwrap();
        let root_content = self.provider.get(PathBuf::from("#"));
        let tree = parser.parse(root_content.as_bytes(), None).unwrap();

        let paths_query = create_key_query("paths");
        let query = Query::new(language, &paths_query).expect("Could not construct query");
        let mut qc = QueryCursor::new();
        let content = root_content.as_bytes();

        let mut entries: Vec<OperationNode> = vec![];
        //let find_keys_and_context =
        //    |key: String, context_node: Node, content: &[u8]| -> Vec<(String, TreeCursor)> {
        //        let query = create_children_keys_query(&key);
        //        let query = Query::new(language, &query).expect("Could not construct query");
        //        let mut qc = QueryCursor::new();
        //        let mut results: Vec<(String, TreeCursor)> = vec![];
        //        for qm in qc.matches(&query, context_node, content) {
        //            if let Some(child_key_capture) = qm.captures.get(2) {
        //                if let Ok(key_text) = child_key_capture.node.utf8_text(content) {
        //                    if let Some(child_value_capture) = qm.captures.get(3) {
        //                        results
        //                            .push((key_text.to_string(), child_value_capture.node.walk()))
        //                    }
        //                }
        //            }
        //        }

        //        results
        //    };
        for qm in qc.matches(&query, tree.root_node(), content).nth(0) {
            if let Some(_) = qm.captures.get(1) {
                let paths_children_query = create_children_keys_query("paths");
                let query =
                    Query::new(language, &paths_children_query).expect("Could not construct query");
                let mut qc = QueryCursor::new();
                let specific_paths: Vec<&str> = qc
                    .matches(&query, tree.root_node(), content)
                    .filter_map(|qm| {
                        println!(
                            "qm? {:?}",
                            qm.captures.get(0).unwrap().node.utf8_text(content)
                        );
                        let pair = (
                            qm.captures.get(2).unwrap().node.utf8_text(content).unwrap(),
                            qm.captures.get(1).unwrap().node,
                        );
                        let mut result = None;
                        let query = create_children_keys_query(pair.0);
                        let query =
                            Query::new(language, &query).expect("Could not construct query");
                        let mut qc = QueryCursor::new();
                        let pairs_for_path: Vec<(&str, Node)> = qc
                            .matches(&query, pair.1, content)
                            .filter_map(|qm| {
                                let mut pair = None;
                                if let Some(cap) = qm.captures.get(2) {
                                    if let Ok(t) = cap.node.utf8_text(content) {
                                        pair = Some((t, qm.captures.get(3).unwrap().node))
                                    }
                                }
                                pair
                            })
                            .collect();
                        if pairs_for_path.iter().any(|pair| pair.0.eq("$ref")) {
                            // follow ref
                            //println!("contains ref {:?}", pair);
                            //let query = create_children_keys_query(key)
                        } else {
                        }

                        //for pair_iter in qm.captures.chunks_exact(2) {
                        //    let pair = match pair_iter {
                        //        &[key, value, ..] => {
                        //            (key.node.utf8_text(content).unwrap(), value.node)
                        //        }
                        //        _ => unreachable!(),
                        //    };
                        //    println!("pair? {:?}", pair);
                        //    println!("value node? {:?}", pair.1.utf8_text(content));

                        //    result = Some(pair.0);
                        //    let query = create_children_query(pair.0);
                        //    let query =
                        //        Query::new(language, &query).expect("Could not construct query");
                        //    let mut qc = QueryCursor::new();
                        //    for qm in qc.matches(&query, pair.1.parent().unwrap(), content) {
                        //        println!(
                        //            "pairs? {:?}",
                        //            qm.captures.get(0).unwrap().node.utf8_text(content)
                        //        )
                        //    }
                        //}
                        result
                    })
                    .collect::<Vec<_>>();

                //let operation_ids: Vec<&str> = specific_paths
                //    .iter()
                //    .filter_map(|path| {
                //        let mut operations: Vec<String> = vec![];
                //        let query = create_children_query(path);
                //        let query =
                //            Query::new(language, &query).expect("Could not construct query");
                //        let mut qc = QueryCursor::new();
                //        for qm in qc.matches(&query, tree.root_node(), content) {

                //        }
                //        Some(operations)
                //    })
                //    .flatten()
                //    .collect();

                //for qm in qc.matches(&query, tree.root_node(), content) {
                //    println!("children {:?}", qm);
                //    if let Some(value_cap) = qm.captures.get(2) {
                //        println!("value_cap {:?}", value_cap.node.utf8_text(content).unwrap());
                //        let operation_query = create_operation_query();
                //        let query = Query::new(language, &operation_query)
                //            .expect("Could not construct query");
                //        let mut qc = QueryCursor::new();

                //        let mut found_operations = false;
                //        for qm in qc.matches(&query, value_cap.node, content) {
                //            found_operations = true;
                //            if let Some(cap) = qm.captures.get(1) {
                //                if let Ok(operation) = cap.node.utf8_text(content) {
                //                    entries.push(OperationNode {
                //                        text: operation.to_string(),
                //                    });
                //                }
                //            }
                //        }
                //    }
                //}
            }
        }
        return entries;
    }
}
fn create_key_query(key: &str) -> String {
    return format!(
        r#"
            (block_mapping_pair key: ((flow_node) @key (eq? @key "{}")) value: (block_node) @value)
            "#,
        key
    );
}
fn create_children_query(key: &str) -> String {
    return format!(
        r#"
            (block_mapping_pair
             key: ((flow_node) @key (eq? @key "{}"))
             value: [
               (flow_node) @value
               (block_node) @value
             ]
             )
            "#,
        key
    );
}

fn create_children_keys_query(key: &str) -> String {
    return format!(
        r#"
            (block_mapping_pair
             key: ((flow_node) @key (eq? @key "{}"))
             value: (block_node (block_mapping (block_mapping_pair key: (flow_node) @childkey value: [
                         (flow_node) @childvalue
                         (block_node) @childvalue
             ])
                     )
                 ) @value
             )
            "#,
        key
    );
}

fn create_operation_query() -> String {
    return format!(
        r#"
            (block_mapping_pair
             key: ((flow_node) @key (eq? @key "operationId"))
             value: [
               (flow_node) @value
               (block_node) @value
             ]
             )
            "#
    );
}

/// The content of the [`node-types.json`][] file for this grammar.
///
/// [`node-types.json`]: https://tree-sitter.github.io/tree-sitter/using-parsers#static-node-types
//pub const NODE_TYPES: &'static str = include_str!("../../../tree_sitter_yaml/src/node-types.json");

// Uncomment these to include any queries that this grammar contains

// pub const HIGHLIGHTS_QUERY: &'static str = include_str!("../../queries/highlights.scm");
// pub const INJECTIONS_QUERY: &'static str = include_str!("../../queries/injections.scm");
// pub const LOCALS_QUERY: &'static str = include_str!("../../queries/locals.scm");
// pub const TAGS_QUERY: &'static str = include_str!("../../queries/tags.scm");

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, error::Error, path::PathBuf};

    use crate::bindings::{ContentProviderMap, OperationParser, TreeSitterOperationParser};

    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(super::language())
            .expect("Error loading yaml language");
    }

    #[test]
    fn test_list() -> Result<(), Box<dyn Error>> {
        let contents = HashMap::from([(
            PathBuf::from("#"),
            r#"
paths:
  /pets:
    delete:
      operationId: deletePets
    get:
      summary: List all pets
      operationId: getPets
    post:
      summary: Create a pet
      operationId: postPets
    trace:
      operationId: tracePets
    options:
      operationId: optionsPets
    put:
      operationId: putPets
    head:
      operationId: headPets
    connect:
      operationId: connectPets

  /pets/{petId}:
    get:
      summary: Info for a specific pet
      operationId: showPetById

                "#
            .to_string(),
        )]);
        let parser = TreeSitterOperationParser::<ContentProviderMap>::new(contents.into());
        let result = parser.get_operation_nodes();
        let node_texts: Vec<String> = result.into_iter().map(|node| node.text).collect();
        assert_eq!(
            vec![
                "deletePets",
                "getPets",
                "postPets",
                "tracePets",
                "optionsPets",
                "putPets",
                "headPets",
                "connectPets",
                "showPetById"
            ],
            node_texts,
            "Returned operations did not match expected operations"
        );
        Ok(())
    }

    #[test]
    fn test_list_multifile() -> Result<(), Box<dyn Error>> {
        let contents = HashMap::from([
            (
                PathBuf::from("#"),
                r#"
paths:
  /pets:
    $ref: paths/pets.yaml
  /pets/{petId}:
    $ref: paths/pet.yaml
                "#
                .to_string(),
            ),
            (
                PathBuf::from("paths/pets.yaml"),
                r#"
// pets.yaml
delete:
  operationId: deletePets
get:
  summary: List all pets
  operationId: getPets
post:
  summary: Create a pet
  operationId: postPets
trace:
  operationId: tracePets
options:
  operationId: optionsPets
put:
  operationId: putPets
head:
  operationId: headPets
connect:
  operationId: connectPets

        "#
                .to_string(),
            ),
            (
                PathBuf::from("paths/pet.yaml"),
                r#"
// pet.yaml
get:
  summary: Info for a specific pet
  operationId: showPetById

        "#
                .to_string(),
            ),
        ]);
        let parser = TreeSitterOperationParser::<ContentProviderMap>::new(contents.into());
        let result = parser.get_operation_nodes();
        let node_texts: Vec<String> = result.into_iter().map(|node| node.text).collect();
        assert_eq!(
            vec![
                "deletePets",
                "getPets",
                "postPets",
                "tracePets",
                "optionsPets",
                "putPets",
                "headPets",
                "connectPets",
                "showPetById"
            ],
            node_texts,
            "Returned operations did not match expected operations"
        );
        Ok(())
    }
}
