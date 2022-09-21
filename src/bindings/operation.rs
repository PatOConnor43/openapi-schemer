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
        let content = self.provider.get_content(PathBuf::from("#"));
        let mut results: Vec<super::OperationNode> = vec![];

        let mut paths_children = get_children_by_key("paths", content.as_bytes());
        if let ChildrenOrRef::Ref(r) = paths_children {
            let content = self.provider.get_content(PathBuf::from(r));
            paths_children = get_top_level_keys(content.as_bytes());
        }
        match paths_children {
            ChildrenOrRef::Ref(_) => panic!("Found $ref when following $ref. Aborting."),
            ChildrenOrRef::Children(children) => {
                for (path, context) in children {
                    let mut methods = get_children_by_key(path.as_ref(), context.as_bytes());
                    if let ChildrenOrRef::Ref(r) = methods {
                        let content = self.provider.get_content(PathBuf::from(r));
                        methods = get_top_level_keys(content.as_bytes());
                    }
                    match methods {
                        ChildrenOrRef::Ref(_) => {
                            panic!("Found $ref when following $ref. Aborting.")
                        }
                        ChildrenOrRef::Children(children) => {
                            for (operation, context) in children {
                                let mut operation_child_keys =
                                    get_children_by_key(operation.as_ref(), context.as_bytes());
                                if let ChildrenOrRef::Ref(r) = operation_child_keys {
                                    let content = self.provider.get_content(PathBuf::from(r));
                                    operation_child_keys = get_top_level_keys(content.as_bytes());
                                }
                                match operation_child_keys {
                                    ChildrenOrRef::Ref(_) => {
                                        panic!("Found $ref when following $ref. Aborting.")
                                    }
                                    ChildrenOrRef::Children(children) => {
                                        // This base case looks pretty gross and maybe it is, but the
                                        // resulting value of children["operationId"] is the string
                                        // "operationId: <whatever>". So I just do some string
                                        // splitting since it should definitely look like that right?
                                        let operation =
                                            children.get("operationId").unwrap().to_string();
                                        let operation = operation
                                            .split("operationId:")
                                            .into_iter()
                                            .last()
                                            .unwrap()
                                            .trim()
                                            .to_owned();
                                        results.push(super::OperationNode { text: operation })
                                    }
                                }
                            }
                        }
                    }
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
                    .unwrap()
                    // Prevent weird file names by removing quotes
                    .replace("'", "")
                    .replace("\"", "");
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

    use mocktopus::mocking::MockResult;
    use mocktopus::mocking::Mockable;

    use crate::bindings::OperationParser;
    use crate::content::ContentProvider;
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
        ContentProviderMap::get_content.mock_safe(move |_, path: PathBuf| {
            MockResult::Return(contents.get(&path).unwrap().to_owned())
        });
        let provider = ContentProviderMap::new();
        let box_provider = Box::new(provider);
        let parser = TreeSitterOperationParser2::new(box_provider);
        let nodes = parser.get_operation_nodes();
        let operation_ids: Vec<String> = nodes.into_iter().map(|node| node.text).collect();
        assert!(operation_ids.contains(&String::from("listPets")));
        assert!(operation_ids.contains(&String::from("createPets")));

        Ok(())
    }

    #[test]
    fn get_operation_nodes_with_ref_operations() -> Result<(), Box<dyn Error>> {
        let root_path = PathBuf::from("#");
        let paths_path = PathBuf::from("Paths.yaml");
        let pets_get_path = PathBuf::from("paths/pets/get.yaml");
        let pets_post_path = PathBuf::from("paths/pets/post.yaml");

        let root_content = r#"
paths:
  $ref: Paths.yaml
            "#;

        let paths_content = r#"
# Paths.yaml
/pets:
    get:
      $ref: paths/pets/get.yaml
    post:
      $ref: paths/pets/post.yaml
            "#;
        let pets_get_content = r#"
# get.yaml
summary: List all pets
operationId: listPets
            "#;
        let pets_post_content = r#"
# post.yaml
summary: Create a pet
operationId: createPets
            "#;
        let contents = HashMap::from([
            (root_path, root_content.to_owned()),
            (paths_path, paths_content.to_owned()),
            (pets_get_path, pets_get_content.to_owned()),
            (pets_post_path, pets_post_content.to_owned()),
        ]);

        ContentProviderMap::get_content.mock_safe(move |_, path: PathBuf| {
            MockResult::Return(contents.get(&path).unwrap().to_owned())
        });

        let provider = ContentProviderMap::new();
        let box_provider = Box::new(provider);
        let parser = TreeSitterOperationParser2::new(box_provider);
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
                assert!(r == "#/fake/ref");
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
                assert_eq!(
                    children.get("test1").unwrap(),
                    "test1:\n    description: yes"
                );
                assert_eq!(
                    children.get("test2").unwrap(),
                    "test2:\n    description: no"
                );
                assert_eq!(
                    children.get("test3").unwrap(),
                    "test3:\n    $ref: '#/fake/ref'"
                );
                Ok(())
            }
            super::ChildrenOrRef::Ref(_) => panic!("Test should have returned Children enum"),
        };
    }
}
