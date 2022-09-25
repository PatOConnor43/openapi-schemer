use std::path::PathBuf;

use crate::content::ContentProvider;

use super::{get_children_by_key, get_top_level_keys, ChildrenOrRef};

#[derive(Clone, Debug)]
pub struct SchemaNode {
    pub text: String,
}

pub trait SchemaParser {
    fn get_schema_nodes(&self) -> Vec<SchemaNode>;
}

pub struct TreeSitterSchemaParser {
    provider: Box<dyn ContentProvider>,
}

impl TreeSitterSchemaParser {
    pub fn new(provider: Box<dyn ContentProvider>) -> Self {
        Self { provider }
    }
}

impl SchemaParser for TreeSitterSchemaParser {
    fn get_schema_nodes(&self) -> Vec<SchemaNode> {
        let content = self.provider.get_content(PathBuf::from("#"));
        let mut results: Vec<SchemaNode> = vec![];

        let mut components_children =
            get_children_by_key("components", content.as_bytes()).unwrap();
        if let ChildrenOrRef::Ref(r) = components_children {
            let content = self.provider.get_content(PathBuf::from(r));
            components_children = get_top_level_keys(content.as_bytes());
        }
        match components_children {
            ChildrenOrRef::Ref(_) => panic!("Found $ref when following $ref. Aborting."),
            ChildrenOrRef::Children(children) => {
                let schemas_context = children
                    .get("schemas")
                    .expect("Did not find schemas within components");
                let schemas_children =
                    get_children_by_key("schemas", schemas_context.as_bytes()).unwrap();
                if let ChildrenOrRef::Ref(_) = schemas_children {
                    panic!("Expected structs under schemas key but found $ref instead.");
                }
                match schemas_children {
                    ChildrenOrRef::Ref(_) => panic!("Found $ref when following $ref. Aborting."),
                    ChildrenOrRef::Children(children) => {
                        for (schema_child, _) in children {
                            results.push(SchemaNode { text: schema_child })
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
    use std::{collections::HashMap, error::Error, path::PathBuf};

    use mocktopus::mocking::*;

    use crate::{
        bindings::schema::{SchemaParser, TreeSitterSchemaParser},
        content::ContentProvider,
        content::ContentProviderMap,
    };

    #[test]
    fn get_schema_nodes_no_refs() -> Result<(), Box<dyn Error>> {
        let root_path = PathBuf::from("#");
        let root_content = r#"
components:
  schemas:
    Pet:
      type: object
      properties:
        id:
          type: integer
          format: int64
        name:
          type: string
        tag:
          type: string
    Pets:
      type: array
      items:
        $ref: '#/components/schemas/Pet'
    Error:
      type: object
      properties:
        code:
          type: integer
          format: int32
        message:
          type: string
            "#;
        let contents = HashMap::from([(root_path, root_content.to_owned())]);

        ContentProviderMap::get_content.mock_safe(move |_, path: PathBuf| {
            MockResult::Return(contents.get(&path).unwrap().to_owned())
        });

        let provider = ContentProviderMap::new();
        let box_provider = Box::new(provider);
        let parser = TreeSitterSchemaParser::new(box_provider);
        let result = parser.get_schema_nodes();
        let node_texts: Vec<String> = result.into_iter().map(|node| node.text).collect();
        assert!(
            node_texts.contains(&String::from("Error")),
            "Missing 'Error' from results"
        );
        assert!(
            node_texts.contains(&String::from("Pet")),
            "Missing 'Pet' from results"
        );
        assert!(
            node_texts.contains(&String::from("Pets")),
            "Missing 'Pets' from results"
        );
        assert_eq!(
            node_texts.len(),
            3,
            "Did not receieve expected number of results"
        );
        Ok(())
    }

    #[test]
    fn get_schema_nodes_ref_schemas() -> Result<(), Box<dyn Error>> {
        let root_path = PathBuf::from("#");
        let schemas_path = PathBuf::from("Schemas.yaml");
        let root_content = r#"
components:
  $ref: 'Schemas.yaml'
            "#;
        let schemas_content = r#"
schemas:
  Pet:
    type: object
    properties:
      id:
        type: integer
        format: int64
      name:
        type: string
      tag:
        type: string
  Pets:
    type: array
    items:
      $ref: '#/components/schemas/Pet'
  Error:
    type: object
    properties:
      code:
        type: integer
        format: int32
      message:
        type: string
          "#;
        let contents = HashMap::from([
            (root_path, root_content.to_owned()),
            (schemas_path, schemas_content.to_owned()),
        ]);

        ContentProviderMap::get_content.mock_safe(move |_, path: PathBuf| {
            MockResult::Return(contents.get(&path).unwrap().to_owned())
        });

        let provider = ContentProviderMap::new();
        let box_provider = Box::new(provider);
        let parser = TreeSitterSchemaParser::new(box_provider);
        let result = parser.get_schema_nodes();
        let node_texts: Vec<String> = result.into_iter().map(|node| node.text).collect();
        assert!(
            node_texts.contains(&String::from("Error")),
            "Missing 'Error' from results"
        );
        assert!(
            node_texts.contains(&String::from("Pet")),
            "Missing 'Pet' from results"
        );
        assert!(
            node_texts.contains(&String::from("Pets")),
            "Missing 'Pets' from results"
        );
        assert_eq!(
            node_texts.len(),
            3,
            "Did not receieve expected number of results"
        );
        Ok(())
    }
}
