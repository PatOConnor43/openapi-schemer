use std::fmt::Display;

use tree_sitter::{Parser, Query, QueryCursor};

use crate::{bindings, error::OpenapiSchemerError};

pub struct ListResult<'a> {
    entries: Vec<&'a str>,
}

impl<'a> ListResult<'a> {
    pub fn new(list: Vec<&str>) -> ListResult<'_> {
        ListResult { entries: list }
    }
}

impl Display for ListResult<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.entries.join("\n"))
    }
}

pub fn list(contents: &str) -> Result<ListResult, OpenapiSchemerError> {
    let mut parser = Parser::new();
    let language = bindings::language();
    parser.set_language(language).unwrap();
    let tree = parser.parse(contents, None).unwrap();

    // This query is an amalgamation of all the different supported http verbs.
    // First we find a key with the text that matches an http verb (get/post/put...),
    // then we look for a child `flow_node` that has the key `operationId`. If we find
    // one we'll try to match the value of that node with should be the name of the
    // operation.
    let query_string = r#"
        (block_mapping_pair
           key: ((flow_node) @delete (eq? @delete "delete"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: (flow_node) @deletevalue (eq? @deletevalue "operationId")
                       value: (flow_node) @deletevalue
                       ))))
        (block_mapping_pair
           key: ((flow_node) @get (eq? @get "get"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: ((flow_node) @operationId (eq? @operationId "operationId"))
                       value: (flow_node) @getvalue
                       ))))
        (block_mapping_pair
           key: ((flow_node) @head (eq? @head "head"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: ((flow_node) @operationId (eq? @operationId "operationId"))
                       value: (flow_node) @headvalue
                       ))))
        (block_mapping_pair
           key: ((flow_node) @options (eq? @options "options"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: ((flow_node) @operationId (eq? @operationId "operationId"))
                       value: (flow_node) @optionsvalue
                       ))))
        (block_mapping_pair
           key: ((flow_node) @patch (eq? @patch "patch"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: ((flow_node) @operationId (eq? @operationId "operationId"))
                       value: (flow_node) @patchvalue
                       ))))
        (block_mapping_pair
           key: ((flow_node) @post (eq? @post "post"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: ((flow_node) @operationId (eq? @operationId "operationId"))
                       value: (flow_node) @postvalue
                       ))))
        (block_mapping_pair
           key: ((flow_node) @put (eq? @put "put"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: ((flow_node) @operationId (eq? @operationId "operationId"))
                       value: (flow_node) @putvalue
                       ))))
        (block_mapping_pair
           key: ((flow_node) @connect (eq? @connect "connect"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: ((flow_node) @operationId (eq? @operationId "operationId"))
                       value: (flow_node) @connectvalue
                       ))))
        (block_mapping_pair
           key: ((flow_node) @trace (eq? @trace "trace"))
           value: (block_node (block_mapping (block_mapping_pair
                       key: ((flow_node) @operationId (eq? @operationId "operationId"))
                       value: (flow_node) @tracevalue
                       ))))
        "#;
    let query = Query::new(language, query_string).expect("Could not construct query");
    let mut qc = QueryCursor::new();
    let provider = contents.as_bytes();

    let mut entries = Vec::new();
    for qm in qc.matches(&query, tree.root_node(), provider) {
        if let Some(cap) = qm.captures.get(2) {
            if let Ok(operation) = cap.node.utf8_text(provider) {
                entries.push(operation);
            }
        }
    }
    Ok(ListResult::new(entries))
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

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
        let result = list(contents)?;
        assert_eq!(
            vec![
                "deletePets",
                "getPets",
                "postPets",
                "tracePets",
                "optionsPets",
                "putPets",
                "headPets",
                "connectPets",
                "showPetById"
            ],
            result.entries,
            "Returned operations did not match expected operations"
        );
        Ok(())
    }
}
