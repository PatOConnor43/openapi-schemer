use std::path::PathBuf;

use tree_sitter::{Parser, Query, QueryCursor};

use crate::content::ContentProvider;

use super::{get_children_by_key, get_top_level_keys, language, ChildrenOrRef};

#[derive(Clone, Debug)]
pub struct SchemaNode {
    pub text: String,
}

pub trait SchemaParser {
    fn get_schema_nodes(&self) -> Vec<SchemaNode>;
}

pub struct TreeSitterSchemaParser<'a> {
    contents: &'a str,
}
impl<'a> TreeSitterSchemaParser<'a> {
    pub fn new(contents: &str) -> TreeSitterSchemaParser {
        TreeSitterSchemaParser { contents }
    }
}
impl<'a> SchemaParser for TreeSitterSchemaParser<'a> {
    fn get_schema_nodes(&self) -> Vec<SchemaNode> {
        let mut parser = Parser::new();
        let language = language();
        parser.set_language(language).unwrap();
        let tree = parser.parse(self.contents, None).unwrap();

        let query_string = r#"
            (block_mapping_pair key: ((flow_node) @key (eq? @key "schemas")) value: (block_node (block_mapping (block_mapping_pair (flow_node) @inner_key))))
        "#;
        let query = Query::new(language, query_string).expect("Could not construct query");
        let mut qc = QueryCursor::new();
        let provider = self.contents.as_bytes();

        let mut entries = Vec::new();
        for qm in qc.matches(&query, tree.root_node(), provider) {
            if let Some(cap) = qm.captures.get(1) {
                if let Ok(route) = cap.node.utf8_text(provider) {
                    entries.push(SchemaNode {
                        text: route.to_string(),
                    });
                }
            }
        }
        return entries;
    }
}

pub struct TreeSitterSchemaParser2 {
    provider: Box<dyn ContentProvider>,
}

impl TreeSitterSchemaParser2 {
    pub fn new(provider: Box<dyn ContentProvider>) -> Self {
        Self { provider }
    }
}

impl SchemaParser for TreeSitterSchemaParser2 {
    fn get_schema_nodes(&self) -> Vec<SchemaNode> {
        let content = self.provider.get_content(PathBuf::from("#"));
        let mut results: Vec<SchemaNode> = vec![];

        let mut components_children = get_children_by_key("components", content.as_bytes());
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
                let mut schemas_children =
                    get_children_by_key("schemas", schemas_context.as_bytes());
                if let ChildrenOrRef::Ref(r) = schemas_children {
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
        bindings::schema::{SchemaParser, TreeSitterSchemaParser, TreeSitterSchemaParser2},
        content::ContentProvider,
        content::ContentProviderMap,
    };

    #[test]
    fn test_list() -> Result<(), Box<dyn Error>> {
        let contents = r#"
components:
  schemas:
    Pet:
      type: object
      required:
        - id
        - name
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
      required:
        - code
        - message
      properties:
        code:
          type: integer
          format: int32
        message:
          type: string

            "#;
        let parser = TreeSitterSchemaParser::new(contents);
        let result = parser.get_schema_nodes();
        let node_texts: Vec<String> = result.into_iter().map(|node| node.text).collect();
        assert_eq!(
            vec!["Pet", "Pets", "Error"],
            node_texts,
            "Returned Schemas did not match expected Schemas"
        );
        Ok(())
    }

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
        let parser = TreeSitterSchemaParser2::new(box_provider);
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
        let parser = TreeSitterSchemaParser2::new(box_provider);
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
