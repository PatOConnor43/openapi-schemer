use std::{collections::HashMap, path::PathBuf};

use tree_sitter::{Parser, Query, QueryCursor};

use crate::content::ContentProvider;

use super::{language, OperationParser};

pub struct TreeSitterOperationParser2 {
    provider: Box<dyn ContentProvider>,
}

impl<'a> TreeSitterOperationParser2 {
    pub fn new(provider: Box<dyn ContentProvider>) -> Self {
        Self { provider }
    }
}

impl OperationParser for TreeSitterOperationParser2 {
    fn get_operation_nodes(&self) -> Vec<super::OperationNode> {
        let content = self.provider.get(PathBuf::from("#"));
        let mut results: Vec<super::OperationNode> = vec![];

        let paths_children = get_children_by_key("paths", content.as_bytes());
        match paths_children {
            ChildrenOrRef::Children(children) => {
                for (path, context) in children {
                    let methods = get_children_by_key(path.as_ref(), context.as_bytes());
                    match methods {
                        ChildrenOrRef::Children(children) => {
                            for (operation, context) in children {
                                let operation = get_operation_id_by_parent(
                                    operation.as_ref(),
                                    context.as_bytes(),
                                )
                                .unwrap();
                                results.push(super::OperationNode { text: operation })
                            }
                        }
                        ChildrenOrRef::Ref(_) => todo!(),
                    }
                }
            }
            ChildrenOrRef::Ref(r) => {
                let content = self.provider.get(PathBuf::from(r));

                let paths_children = get_top_level_keys(content.as_bytes());
                match paths_children {
                    ChildrenOrRef::Children(children) => {
                        for (path, context) in children {
                            let methods = get_children_by_key(path.as_ref(), context.as_bytes());
                            match methods {
                                ChildrenOrRef::Children(children) => {
                                    for (operation, context) in children {
                                        let operation = get_operation_id_by_parent(
                                            operation.as_ref(),
                                            context.as_bytes(),
                                        )
                                        .unwrap();
                                        results.push(super::OperationNode { text: operation })
                                    }
                                }
                                ChildrenOrRef::Ref(_) => todo!(),
                            }
                        }
                    }
                    ChildrenOrRef::Ref(_) => panic!("Found $ref when following $ref. Aborting."),
                }
            }
        }

        results
    }
}

fn get_top_level_keys(content: &[u8]) -> ChildrenOrRef {
    let language = language();
    let mut parser = Parser::new();
    parser.set_language(language).unwrap();
    let tree = parser.parse(content.to_owned(), None).unwrap();
    let query = super::create_top_level_yaml_context_query();
    let query = Query::new(language, &query).expect("Could not construct query");
    let mut qc = QueryCursor::new();

    let mut results: HashMap<String, String> = HashMap::new();

    for qm in qc.matches(&query, tree.root_node(), content) {
        let child_key_index = query.capture_index_for_name("child-key").unwrap();
        let child_context_index = query.capture_index_for_name("child-context").unwrap();
        let child_key_node = qm.nodes_for_capture_index(child_key_index).last().unwrap();
        if let Ok(key_text) = child_key_node.utf8_text(content) {
            let parent_context_node = qm
                .nodes_for_capture_index(child_context_index)
                .last()
                .unwrap();
            results.insert(
                key_text.to_string(),
                parent_context_node.utf8_text(content).unwrap().to_string(),
            );
        }
    }

    ChildrenOrRef::Children(results)
}

fn get_operation_id_by_parent(parent: &str, content: &[u8]) -> Option<String> {
    let language = language();
    let mut parser = Parser::new();
    parser.set_language(language).unwrap();
    let tree = parser.parse(content.to_owned(), None).unwrap();
    let query = super::create_yaml_context_query(parent);
    let query = Query::new(language, &query).expect("Could not construct query");
    let mut qc = QueryCursor::new();
    for qm in qc.matches(&query, tree.root_node(), content) {
        let child_key_index = query.capture_index_for_name("child-key").unwrap();
        let child_value_index = query.capture_index_for_name("child-value").unwrap();
        let child_key_node = qm.nodes_for_capture_index(child_key_index).last().unwrap();
        if let Ok(key_text) = child_key_node.utf8_text(content) {
            if key_text == "operationId" {
                let child_value_node_text = qm
                    .nodes_for_capture_index(child_value_index)
                    .last()
                    .unwrap()
                    .utf8_text(content)
                    .unwrap();
                return Some(child_value_node_text.to_owned());
            }
        }
    }

    None
}

fn get_children_by_key(key: &str, content: &[u8]) -> ChildrenOrRef {
    let language = language();
    let mut parser = Parser::new();
    parser.set_language(language).unwrap();
    let tree = parser.parse(content.to_owned(), None).unwrap();
    let query = super::create_yaml_context_query(key);
    let query = Query::new(language, &query).expect("Could not construct query");
    let mut qc = QueryCursor::new();

    let mut results: HashMap<String, String> = HashMap::new();

    for qm in qc.matches(&query, tree.root_node(), content) {
        let child_key_index = query.capture_index_for_name("child-key").unwrap();
        let child_value_index = query.capture_index_for_name("child-value").unwrap();
        let child_context_index = query.capture_index_for_name("child-context").unwrap();
        let child_key_node = qm.nodes_for_capture_index(child_key_index).last().unwrap();
        if let Ok(key_text) = child_key_node.utf8_text(content) {
            if key_text == "$ref" {
                let child_value_node_text = qm
                    .nodes_for_capture_index(child_value_index)
                    .last()
                    .unwrap()
                    .utf8_text(content)
                    .unwrap();
                return ChildrenOrRef::Ref(child_value_node_text.to_owned());
            }
            let parent_context_node = qm
                .nodes_for_capture_index(child_context_index)
                .last()
                .unwrap();
            results.insert(
                key_text.to_string(),
                parent_context_node.utf8_text(content).unwrap().to_string(),
            );
        }
    }

    ChildrenOrRef::Children(results)
}

#[derive(Debug)]
enum ChildrenOrRef {
    Children(HashMap<String, String>),
    Ref(String),
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::error::Error;
    use std::path::PathBuf;

    use crate::bindings::OperationParser;
    use crate::content::ContentProviderMap;

    use super::TreeSitterOperationParser2;

    #[test]
    fn get_operation_nodes() -> Result<(), Box<dyn Error>> {
        let root_path = PathBuf::from("#");
        let root_content = r#"
paths:
  /pets:
    get:
      summary: List all pets
      operationId: listPets
    post:
      summary: Create a pet
      operationId: createPets
            "#;
        let contents = HashMap::from([(root_path, root_content.to_owned())]);
        let provider = Box::new(ContentProviderMap::from_map(contents));
        let parser = TreeSitterOperationParser2::new(provider);
        let nodes = parser.get_operation_nodes();
        let operation_ids: Vec<String> = nodes.into_iter().map(|node| node.text).collect();
        assert!(operation_ids.contains(&String::from("listPets")));
        assert!(operation_ids.contains(&String::from("createPets")));

        Ok(())
    }

    #[test]
    fn get_operation_nodes_with_ref() -> Result<(), Box<dyn Error>> {
        let root_path = PathBuf::from("#");
        let paths_path = PathBuf::from("Paths.yaml");

        let root_content = r#"
paths:
  $ref: Paths.yaml
            "#;

        let paths_content = r#"
# Paths.yaml
/pets:
    get:
      summary: List all pets
      operationId: listPets
    post:
      summary: Create a pet
      operationId: createPets
            "#;
        let contents = HashMap::from([
            (root_path, root_content.to_owned()),
            (paths_path, paths_content.to_owned()),
        ]);
        let provider = Box::new(ContentProviderMap::from_map(contents));
        let parser = TreeSitterOperationParser2::new(provider);
        let nodes = parser.get_operation_nodes();
        let operation_ids: Vec<String> = nodes.into_iter().map(|node| node.text).collect();
        assert!(operation_ids.contains(&String::from("listPets")));
        assert!(operation_ids.contains(&String::from("createPets")));

        Ok(())
    }

    #[test]
    fn get_children_by_key_no_ref() -> Result<(), Box<dyn Error>> {
        let results = super::get_children_by_key(
            "test",
            r#"
test:
  test1:
    description: yes
  test2:
    description: no"#
                .as_bytes(),
        );
        return match results {
            super::ChildrenOrRef::Children(children) => {
                assert!(children.get("test1").unwrap() == "test1:\n    description: yes");
                assert!(children.get("test2").unwrap() == "test2:\n    description: no");
                Ok(())
            }
            super::ChildrenOrRef::Ref(_) => panic!("Test should have returned Children enum"),
        };
    }

    #[test]
    fn get_children_by_key_ref() -> Result<(), Box<dyn Error>> {
        let results = super::get_children_by_key(
            "test",
            r#"
test:
  test1:
    description: yes
  test2:
    description: no
  $ref: '#/fake/ref'"#
                .as_bytes(),
        );
        return match results {
            super::ChildrenOrRef::Children(_) => {
                panic!("Test should have returned Ref enum")
            }
            super::ChildrenOrRef::Ref(r) => {
                assert!(r == "'#/fake/ref'");
                Ok(())
            }
        };
    }

    #[test]
    fn get_children_by_key_child_contains_ref() -> Result<(), Box<dyn Error>> {
        let results = super::get_children_by_key(
            "test",
            r#"
test:
  test1:
    description: yes
  test2:
    description: no
  test3:
    $ref: '#/fake/ref'"#
                .as_bytes(),
        );
        return match results {
            super::ChildrenOrRef::Children(children) => {
                assert!(children.get("test1").unwrap() == "test1:\n    description: yes");
                assert!(children.get("test2").unwrap() == "test2:\n    description: no");
                assert!(children.get("test3").unwrap() == "test3:\n    $ref: '#/fake/ref'");
                Ok(())
            }
            super::ChildrenOrRef::Ref(_) => panic!("Test should have returned Children enum"),
        };
    }
}
