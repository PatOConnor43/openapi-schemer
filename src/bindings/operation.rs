use std::path::PathBuf;

use crate::content::ContentProvider;

use super::{get_children_by_key, get_top_level_keys, ChildrenOrRef, OperationParser};

pub struct TreeSitterOperationParser {
    provider: Box<dyn ContentProvider>,
}

impl TreeSitterOperationParser {
    pub fn new(provider: Box<dyn ContentProvider>) -> Self {
        Self { provider }
    }
}

impl OperationParser for TreeSitterOperationParser {
    fn get_operation_nodes(&self) -> Vec<super::OperationNode> {
        let content = self.provider.get_content(PathBuf::from("#"));
        let mut results: Vec<super::OperationNode> = vec![];

        let mut paths_children = get_children_by_key("paths", content.as_bytes()).unwrap();
        if let ChildrenOrRef::Ref(r) = paths_children {
            let content = self.provider.get_content(PathBuf::from(r));
            paths_children = get_top_level_keys(content.as_bytes());
        }
        match paths_children {
            super::ChildrenOrRef::Ref(_) => panic!("Found $ref when following $ref. Aborting."),
            super::ChildrenOrRef::Children(children) => {
                for (path, context) in children {
                    let mut methods =
                        get_children_by_key(path.as_ref(), context.as_bytes()).unwrap();
                    if let ChildrenOrRef::Ref(r) = methods {
                        let content = self.provider.get_content(PathBuf::from(r));
                        methods = get_top_level_keys(content.as_bytes());
                    }
                    match methods {
                        super::ChildrenOrRef::Ref(_) => {
                            panic!("Found $ref when following $ref. Aborting.")
                        }
                        super::ChildrenOrRef::Children(children) => {
                            for (operation, context) in children {
                                let mut operation_child_keys =
                                    get_children_by_key(operation.as_ref(), context.as_bytes())
                                        .unwrap();
                                if let ChildrenOrRef::Ref(r) = operation_child_keys {
                                    let content = self.provider.get_content(PathBuf::from(r));
                                    operation_child_keys = get_top_level_keys(content.as_bytes());
                                }
                                match operation_child_keys {
                                    super::ChildrenOrRef::Ref(_) => {
                                        panic!("Found $ref when following $ref. Aborting.")
                                    }
                                    super::ChildrenOrRef::Children(children) => {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::error::Error;
    use std::path::PathBuf;

    use mocktopus::mocking::MockResult;
    use mocktopus::mocking::Mockable;

    use crate::bindings::{ChildrenOrRef, OperationParser};
    use crate::content::ContentProvider;
    use crate::content::ContentProviderMap;

    use super::TreeSitterOperationParser;

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
        let parser = TreeSitterOperationParser::new(provider);
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
        let parser = TreeSitterOperationParser::new(box_provider);
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
        let parser = TreeSitterOperationParser::new(box_provider);
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
        )
        .unwrap();
        return match results {
            ChildrenOrRef::Children(children) => {
                assert!(children.get("test1").unwrap() == "test1:\n    description: yes");
                assert!(children.get("test2").unwrap() == "test2:\n    description: no");
                Ok(())
            }
            ChildrenOrRef::Ref(_) => panic!("Test should have returned Children enum"),
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
        )
        .unwrap();
        return match results {
            ChildrenOrRef::Children(_) => {
                panic!("Test should have returned Ref enum")
            }
            ChildrenOrRef::Ref(r) => {
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
        )
        .unwrap();
        return match results {
            ChildrenOrRef::Children(children) => {
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
            ChildrenOrRef::Ref(_) => panic!("Test should have returned Children enum"),
        };
    }
}
