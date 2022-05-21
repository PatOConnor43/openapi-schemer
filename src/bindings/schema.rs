use tree_sitter::{Parser, Query, QueryCursor};

use super::language;

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

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::bindings::schema::{SchemaParser, TreeSitterSchemaParser};

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
}
