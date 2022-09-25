use anyhow::Result;
use std::path::PathBuf;

use crate::content::ContentProvider;

use super::{get_children_by_key, get_top_level_keys, ChildrenOrRef};

#[derive(Clone, Debug)]
pub struct PathNode {
    pub text: String,
}

pub trait PathParser {
    fn get_path_nodes(&self) -> Result<Vec<PathNode>>;
}

pub struct TreeSitterPathParser {
    provider: Box<dyn ContentProvider>,
}

impl TreeSitterPathParser {
    pub fn new(provider: Box<dyn ContentProvider>) -> Self {
        Self { provider }
    }
}

impl PathParser for TreeSitterPathParser {
    fn get_path_nodes(&self) -> Result<Vec<PathNode>> {
        let content = self.provider.get_content(PathBuf::from("#"));
        let mut results: Vec<PathNode> = vec![];

        let mut paths_children = get_children_by_key("paths", content.as_bytes()).unwrap();
        if let ChildrenOrRef::Ref(r) = paths_children {
            let content = self.provider.get_content(PathBuf::from(r));
            paths_children = get_top_level_keys(content.as_bytes()).unwrap();
        }
        match paths_children {
            super::ChildrenOrRef::Ref(_) => panic!("Found $ref when following $ref. Aborting."),
            super::ChildrenOrRef::Children(children) => {
                for (path, _) in children {
                    results.push(PathNode { text: path })
                }
            }
        }
        Ok(results)
    }
}
#[cfg(test)]
mod tests {
    use std::{collections::HashMap, error::Error, path::PathBuf};

    use mocktopus::mocking::*;

    use crate::{
        bindings::path::{PathParser, TreeSitterPathParser},
        content::ContentProvider,
        content::ContentProviderMap,
    };

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
        let parser = TreeSitterPathParser::new(box_provider);
        let nodes = parser.get_path_nodes().unwrap();
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
        let parser = TreeSitterPathParser::new(box_provider);
        let nodes = parser.get_path_nodes().unwrap();
        let paths: Vec<String> = nodes.into_iter().map(|node| node.text).collect();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&String::from("/pets")));
        assert!(paths.contains(&String::from("/pets/{petId}")));

        Ok(())
    }
}
