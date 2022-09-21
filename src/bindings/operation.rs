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

        let refs_query = super::create_key_query("paths");
        let mut parser = Parser::new();
        let language = language();
        parser.set_language(language).unwrap();
        let tree = parser.parse(content.to_owned(), None).unwrap();
        let query = Query::new(language, &refs_query).expect("Could not construct query");
        let mut qc = QueryCursor::new();
        let provider = content.as_bytes();

        for qm in qc.matches(&query, tree.root_node(), provider) {
            for cap in qm.captures {
                if query.capture_names()[cap.index as usize] == "query-value" {
                    let paths_query = Query::new(
                        language,
                        r#"
                      (
                        (block_mapping_pair
                         key: (flow_node) @key-name
                         value: (
                           block_node (
                             block_mapping (
                               block_mapping_pair key: (flow_node) @paths-child-value
                             )
                           )
                         )
                        )
                        (#eq? @key-name "paths")
                      )
                    "#,
                    )
                    .unwrap();
                    let mut qc = QueryCursor::new();
                    let mut paths: Vec<String> = Vec::new();
                    for qm in qc.matches(&paths_query, tree.root_node(), provider) {
                        for cap in qm.captures {
                            if paths_query.capture_names()[cap.index as usize]
                                == "paths-child-value"
                            {
                                paths.push(cap.node.utf8_text(provider).unwrap().to_owned())
                            }
                        }
                    }
                    println!("{:?}", paths);
                    if paths.contains(&String::from("$ref")) {
                        // Open the ref'd content from the provider
                        todo!()
                    } else {
                        let methods_by_path = get_methods_by_path(&paths, content.to_owned());
                        println!("{:?}", methods_by_path);
                    }
                }
            }
        }

        vec![]
    }
}

fn get_methods_by_path(paths: &[String], content: String) -> HashMap<String, Vec<String>> {
    let mut results: HashMap<String, Vec<String>> = HashMap::new();

    let mut parser = Parser::new();
    let language = language();
    parser.set_language(language).unwrap();
    let tree = parser.parse(content.to_owned(), None).unwrap();

    for path in paths {
        // Add a default vec
        results.insert(path.to_owned(), Vec::new());

        let query = super::create_children_keys_query(path);
        let query = Query::new(language, &query).expect("Could not construct query");
        let mut qc = QueryCursor::new();
        let provider = content.as_bytes();

        for qm in qc.matches(&query, tree.root_node(), provider) {
            for cap in qm.captures {
                if query.capture_names()[cap.index as usize] == "child-key" {
                    let text = cap.node.utf8_text(provider).unwrap();
                    if [
                        "get", "post", "put", "delete", "options", "head", "patch", "connect",
                        "trace",
                    ]
                    .contains(&text)
                    {
                        results.get_mut(path).unwrap().push(text.to_owned())
                    }
                }
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::error::Error;
    use std::path::PathBuf;

    use mocktopus::mocking::*;

    use crate::bindings::OperationParser;
    use crate::content::ContentProviderMap;

    use super::TreeSitterOperationParser2;

    #[test]
    fn get_operation_nodes() -> Result<(), Box<dyn Error>> {
        let root_path = PathBuf::from("#");
        let root_content = r#"
servers:
  - url: http://petstore.swagger.io/v1
paths:
  /pets:
    get:
      summary: List all pets
      operationId: listPets
      tags:
        - pets
      parameters:
        - name: limit
          in: query
          description: How many items to return at one time (max 100)
          required: false
          schema:
            type: integer
            format: int32
      responses:
        '200':
          description: A paged array of pets
          headers:
            x-next:
              description: A link to the next page of responses
              schema:
                type: string
          content:
            application/json:    
              schema:
                $ref: '#/components/schemas/Pets'
        default:
          description: unexpected error
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Error'
    post:
      summary: Create a pet
      operationId: createPets
      tags:
        - pets
      responses:
        '201':
          description: Null response
        default:
          description: unexpected error
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Error'
            "#;
        let contents = HashMap::from([(root_path, root_content.to_owned())]);
        let provider = Box::new(ContentProviderMap::from_map(contents));
        let parser = TreeSitterOperationParser2::new(provider);
        let nodes = parser.get_operation_nodes();
        println!("{:?}", nodes);

        Ok(())
    }
}
