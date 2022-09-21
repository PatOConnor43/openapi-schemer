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
use tree_sitter::{Language, Parser, Query, QueryCursor};

#[cfg(test)]
use mocktopus::macros::mockable;

extern "C" {
    fn tree_sitter_yaml() -> Language;
}

/// Get the tree-sitter [Language][] for this grammar.
///
/// [Language]: https://docs.rs/tree-sitter/*/tree_sitter/struct.Language.html
pub fn language() -> Language {
    unsafe { tree_sitter_yaml() }
}

#[cfg_attr(test, mockable)]
pub fn find_refs(content: &str) -> Vec<String> {
    let mut results: Vec<String> = vec![];

    let refs_query = create_ref_query();
    let mut parser = Parser::new();
    let language = language();
    parser.set_language(language).unwrap();
    let tree = parser.parse(content, None).unwrap();
    let query = Query::new(language, &refs_query).expect("Could not construct query");
    let mut qc = QueryCursor::new();
    let provider = content.as_bytes();

    for qm in qc.matches(&query, tree.root_node(), provider) {
        for cap in qm.captures {
            if query.capture_names()[cap.index as usize] == "query-value" {
                if let Ok(text) = cap.node.utf8_text(provider) {
                    results.push(text.replace("'", "").replace("\"", ""));
                }
            }
        }
    }
    results
}

fn create_ref_query() -> String {
    // Values can either be `block_node` or `flow_node`. It seems like if the
    // child doesn't have children it's a `flow_node`. Since `$ref` should never
    // have children it will always be a `flow_node`. Something like `components`
    // would probably be a block_node.

    return format!(
        r#"
            (block_mapping_pair key: ((flow_node) @query-key (#eq? @query-key "$ref")) value: (flow_node) @query-value)
            "#
    );
}

fn create_key_query(key: &str) -> String {
    // Allow values to be block or flow. Callers can disambiguate by the type if they care.
    return format!(
        r#"
            (block_mapping_pair key: ((flow_node) @query-key (#eq? @query-key "{}")) value: [(flow_node)(block_node)] @query-value)
            "#,
        key
    );
}

fn create_children_keys_query(parent_key: &str) -> String {
    return format!(
        r#"
        (
            (block_mapping_pair
             key: (flow_node) @key-name
             value: (
                 block_node (
                     block_mapping (
                         block_mapping_pair
                         key: (flow_node) @child-key
                         value: [(flow_node)(block_node)] @child-value
                     )
                 )
             ) @key-content
            )
            (#eq? @key-name "{}")
        )
        "#,
        parent_key
    );
}

#[derive(Clone, Debug)]
pub struct OperationNode {
    pub text: String,
}

pub trait OperationParser {
    fn get_operation_nodes(&self) -> Vec<OperationNode>;
}

pub struct TreeSitterOperationParser<'a> {
    contents: &'a str,
}
impl<'a> TreeSitterOperationParser<'a> {
    pub fn new(contents: &str) -> TreeSitterOperationParser {
        TreeSitterOperationParser { contents }
    }
}
impl<'a> OperationParser for TreeSitterOperationParser<'a> {
    fn get_operation_nodes(&self) -> Vec<OperationNode> {
        let mut parser = Parser::new();
        let language = language();
        parser.set_language(language).unwrap();
        let tree = parser.parse(self.contents, None).unwrap();

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
        let provider = self.contents.as_bytes();

        let mut entries = Vec::new();
        for qm in qc.matches(&query, tree.root_node(), provider) {
            if let Some(cap) = qm.captures.get(2) {
                if let Ok(operation) = cap.node.utf8_text(provider) {
                    entries.push(OperationNode {
                        text: operation.to_string(),
                    });
                }
            }
        }
        return entries;
    }
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
    use std::error::Error;

    use crate::bindings::{OperationParser, TreeSitterOperationParser};

    use super::{create_key_query, find_refs};

    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(super::language())
            .expect("Error loading yaml language");
    }

    #[test]
    fn test_list() -> Result<(), Box<dyn Error>> {
        let contents = r#"
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

            "#;
        let parser = TreeSitterOperationParser::new(contents);
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
    fn test_find_refs() -> Result<(), Box<dyn Error>> {
        let content = r#"
openapi: '3.0.0'
info:
  version: 1.0.0
  title: Swagger Petstore
  description: Multi-file boilerplate for OpenAPI Specification.
  license:
    name: MIT
  contact:
    name: API Support
    url: http://www.example.com/support
    email: support@example.com
servers:
  - url: http://petstore.swagger.io/v1
tags:
  - name: pets
paths:
  /pets:
    $ref: 'resources/pets.yaml'
  /pets/{petId}:
    $ref: 'resources/pet.yaml'
                "#;
        let refs = find_refs(content);
        assert_eq!(refs.len(), 2);
        assert!(refs.contains(&String::from("resources/pets.yaml")));
        assert!(refs.contains(&String::from("resources/pet.yaml")));

        Ok(())
    }
}
