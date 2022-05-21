use tree_sitter::{Parser, Query, QueryCursor};

use super::language;

#[derive(Clone, Debug)]
pub struct PathNode {
    pub text: String,
}

pub trait PathParser {
    fn get_path_nodes(&self) -> Vec<PathNode>;
}

pub struct TreeSitterPathParser<'a> {
    contents: &'a str,
}
impl<'a> TreeSitterPathParser<'a> {
    pub fn new(contents: &str) -> TreeSitterPathParser {
        TreeSitterPathParser { contents }
    }
}
impl<'a> PathParser for TreeSitterPathParser<'a> {
    fn get_path_nodes(&self) -> Vec<PathNode> {
        let mut parser = Parser::new();
        let language = language();
        parser.set_language(language).unwrap();
        let tree = parser.parse(self.contents, None).unwrap();

        let query_string = r#"
            (block_mapping_pair key: ((flow_node) @key (eq? @key "paths")) value: (block_node (block_mapping (block_mapping_pair (flow_node) @inner_key))))
        "#;
        let query = Query::new(language, query_string).expect("Could not construct query");
        let mut qc = QueryCursor::new();
        let provider = self.contents.as_bytes();

        let mut entries = Vec::new();
        for qm in qc.matches(&query, tree.root_node(), provider) {
            if let Some(cap) = qm.captures.get(1) {
                if let Ok(route) = cap.node.utf8_text(provider) {
                    entries.push(PathNode {
                        text: route.to_string(),
                    });
                }
            }
        }
        return entries;
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::bindings::path::{PathParser, TreeSitterPathParser};

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
        let parser = TreeSitterPathParser::new(contents);
        let result = parser.get_path_nodes();
        let node_texts: Vec<String> = result.into_iter().map(|node| node.text).collect();
        assert_eq!(
            vec!["/pets", "/pets/{petId}"],
            node_texts,
            "Returned paths did not match expected paths"
        );
        Ok(())
    }
}
