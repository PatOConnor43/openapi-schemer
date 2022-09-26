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

pub mod operation;
pub mod path;
pub mod schema;

use anyhow::{Context, Error, Result};
use std::collections::HashMap;

use tree_sitter::{Language, Parser, Query, QueryCursor};

#[cfg(test)]
use mocktopus::macros::mockable;

use crate::error::OpenapiSchemerError;

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

fn create_top_level_yaml_context_query() -> String {
    return format!(
        r#"
        (document
         (block_node
          (block_mapping
           (block_mapping_pair
            key: (flow_node) @child-key
            value: [(flow_node)(block_node)] @child-value
           ) @child-context
          )
         )
        )
        "#
    );
}

fn create_yaml_context_query(parent_key: &str) -> String {
    return format!(
        r#"
        (
            (block_mapping_pair
             key: (flow_node) @parent-key
             value: (
                 block_node (
                     block_mapping (
                         block_mapping_pair
                         key: (flow_node) @child-key
                         value: [(flow_node)(block_node)] @child-value
                     ) @child-context
                 )
             ) @parent-value
            ) @parent-context
            (#eq? @parent-key "{}")
        )
        "#,
        parent_key
    );
}

fn get_top_level_keys(content: &[u8]) -> Result<ChildrenOrRef> {
    let language = language();
    let mut parser = Parser::new();
    parser.set_language(language)?;
    let tree = parser
        .parse(content.to_owned(), None)
        .ok_or_else(|| Error::msg(format!("Could not parse tree")))?;
    let query = create_top_level_yaml_context_query();
    let query = Query::new(language, &query)
        .with_context(|| format!("Could not construct query `{}`", query))?;
    let mut qc = QueryCursor::new();

    let mut results: HashMap<String, String> = HashMap::new();

    for qm in qc.matches(&query, tree.root_node(), content) {
        let child_key_index = query
            .capture_index_for_name("child-key")
            .ok_or_else(|| Error::msg(format!("Could not find capture for `{}`", "child-key")))?;
        let child_context_index =
            query
                .capture_index_for_name("child-context")
                .ok_or_else(|| {
                    Error::msg(format!("Could not find capture for `{}`", "child-context"))
                })?;
        let child_key_node = qm
            .nodes_for_capture_index(child_key_index)
            .last()
            .ok_or_else(|| Error::msg(format!("Could not find node for `{}`", "child-key")))?;
        if let Ok(key_text) = child_key_node.utf8_text(content) {
            let child_context_node = qm
                .nodes_for_capture_index(child_context_index)
                .last()
                .ok_or_else(|| {
                    Error::msg(format!("Could not find node for `{}`", "child-context"))
                })?;
            results.insert(
                key_text.to_string(),
                child_context_node
                    .utf8_text(content)
                    .with_context(|| {
                        format!("Could not extract value text for key `{}`", key_text)
                    })?
                    .to_string(),
            );
        }
    }

    Ok(ChildrenOrRef::Children(results))
}

fn get_children_by_key(key: &str, content: &[u8]) -> Result<ChildrenOrRef> {
    let language = language();
    let mut parser = Parser::new();
    parser.set_language(language)?;
    let tree = parser
        .parse(content.to_owned(), None)
        .ok_or_else(|| Error::msg(format!("Could not parse tree for parent key `{}`", key)))?;
    let query = create_yaml_context_query(key);
    let query = Query::new(language, &query)
        .with_context(|| format!("Could not construct query `{}`", query))?;
    let mut qc = QueryCursor::new();

    let mut results: HashMap<String, String> = HashMap::new();

    for qm in qc.matches(&query, tree.root_node(), content) {
        let child_key_index = query
            .capture_index_for_name("child-key")
            .ok_or_else(|| Error::msg(format!("Could not find capture for `{}`", "child-key")))?;
        let child_value_index = query
            .capture_index_for_name("child-value")
            .ok_or_else(|| Error::msg(format!("Could not find capture for `{}`", "child-value")))?;
        let child_context_index =
            query
                .capture_index_for_name("child-context")
                .ok_or_else(|| {
                    Error::msg(format!("Could not find capture for `{}`", "child-context"))
                })?;
        let child_key_node = qm
            .nodes_for_capture_index(child_key_index)
            .last()
            .ok_or_else(|| {
                Error::msg(format!("Could not find node for `{}`", "child-key-index"))
            })?;
        if let Ok(key_text) = child_key_node.utf8_text(content) {
            if key_text == "$ref" {
                let child_value_node_text = qm
                    .nodes_for_capture_index(child_value_index)
                    .last()
                    .ok_or_else(|| {
                        Error::msg(format!("Could not find node for `{}`", "child-value-index"))
                    })?
                    .utf8_text(content)
                    .with_context(|| format!("Could not extract path for $ref node"))?
                    // Prevent weird file names by removing quotes
                    .replace("'", "")
                    .replace("\"", "");
                return Ok(ChildrenOrRef::Ref(child_value_node_text.to_owned()));
            }
            let child_context_node = qm
                .nodes_for_capture_index(child_context_index)
                .last()
                .ok_or_else(|| {
                    Error::msg(format!("Could not find node for `{}`", "child-context"))
                })?;
            results.insert(
                key_text.to_string(),
                child_context_node
                    .utf8_text(content)
                    .with_context(|| format!("Could not extract text for child context"))?
                    .to_string(),
            );
        }
    }

    Ok(ChildrenOrRef::Children(results))
}

#[derive(Debug)]
pub enum ChildrenOrRef {
    Children(HashMap<String, String>),
    Ref(String),
}

#[derive(Clone, Debug)]
pub struct OperationNode {
    pub text: String,
}

pub trait OperationParser {
    fn get_operation_nodes(&self) -> Result<Vec<OperationNode>, OpenapiSchemerError>;
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::find_refs;

    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(super::language())
            .expect("Error loading yaml language");
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
