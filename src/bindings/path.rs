use std::path::PathBuf;

use tree_sitter::{Parser, Query, QueryCursor};

use crate::content::ContentProvider;

use super::{get_children_by_key, get_top_level_keys, language, ChildrenOrRef};

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

pub struct TreeSitterPathParser2 {
    provider: Box<dyn ContentProvider>,
}

impl TreeSitterPathParser2 {
    pub fn new(provider: Box<dyn ContentProvider>) -> Self {
        Self { provider }
    }
}

impl PathParser for TreeSitterPathParser2 {
    fn get_path_nodes(&self) -> Vec<PathNode> {
        let content = self.provider.get_content(PathBuf::from("#"));
        let mut results: Vec<PathNode> = vec![];

        let mut paths_children = get_children_by_key("paths", content.as_bytes());
        if let ChildrenOrRef::Ref(r) = paths_children {
            let content = self.provider.get_content(PathBuf::from(r));
            paths_children = get_top_level_keys(content.as_bytes());
        }
        match paths_children {
            super::ChildrenOrRef::Ref(_) => panic!("Found $ref when following $ref. Aborting."),
            super::ChildrenOrRef::Children(children) => {
                for (path, _) in children {
                    results.push(PathNode { text: path })
                }
            }
        }
        results
    }
}
#[cfg(test)]
mod tests {
    use std::{collections::HashMap, error::Error, path::PathBuf};

    use mocktopus::mocking::*;

    use crate::{
        bindings::path::{PathParser, TreeSitterPathParser, TreeSitterPathParser2},
        content::ContentProvider,
        content::ContentProviderMap,
    };

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

    #[test]
    fn get_path_nodes_no_ref() -> Result<(), Box<dyn Error>> {
        let root_path = PathBuf::from("#");
        let root_content = r#"
paths:
  /pets:
    get:
      summary: List all pets
      operationId: getPets
  /pets/{petId}:
    get:
      summary: Info for a specific pet
      operationId: showPetById
"#;
        let contents = HashMap::from([(root_path, root_content.to_owned())]);
        ContentProviderMap::get_content.mock_safe(move |_, path: PathBuf| {
            MockResult::Return(contents.get(&path).unwrap().to_owned())
        });
        let provider = ContentProviderMap::new();
        let box_provider = Box::new(provider);
        let parser = TreeSitterPathParser2::new(box_provider);
        let nodes = parser.get_path_nodes();
        let paths: Vec<String> = nodes.into_iter().map(|node| node.text).collect();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&String::from("/pets")));
        assert!(paths.contains(&String::from("/pets/{petId}")));

        Ok(())
    }

    #[test]
    fn get_path_nodes_ref() -> Result<(), Box<dyn Error>> {
        let root_path = PathBuf::from("#");
        let paths_path = PathBuf::from("Paths.yaml");

        let root_content = r#"
paths:
  $ref: 'Paths.yaml'
"#;
        let paths_content = r#"
/pets:
  get:
    summary: List all pets
    operationId: getPets
/pets/{petId}:
  get:
    summary: Info for a specific pet
    operationId: showPetById
"#;
        let contents = HashMap::from([
            (root_path, root_content.to_owned()),
            (paths_path, paths_content.to_owned()),
        ]);
        ContentProviderMap::get_content.mock_safe(move |_, path: PathBuf| {
            MockResult::Return(contents.get(&path).unwrap().to_owned())
        });
        let provider = ContentProviderMap::new();
        let box_provider = Box::new(provider);
        let parser = TreeSitterPathParser2::new(box_provider);
        let nodes = parser.get_path_nodes();
        let paths: Vec<String> = nodes.into_iter().map(|node| node.text).collect();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&String::from("/pets")));
        assert!(paths.contains(&String::from("/pets/{petId}")));

        Ok(())
    }
}
