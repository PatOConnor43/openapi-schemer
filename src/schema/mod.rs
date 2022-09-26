use std::fmt::Display;

use crate::{bindings::schema::SchemaParser, error::OpenapiSchemerError};

#[derive(Debug, PartialEq, Eq)]
pub struct ListResult {
    entries: Vec<String>,
}

impl ListResult {
    pub fn new(list: Vec<String>) -> ListResult {
        ListResult { entries: list }
    }
}

impl Display for ListResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.entries.join("\n"))
    }
}

pub fn list<T: SchemaParser>(parser: T) -> Result<ListResult, OpenapiSchemerError> {
    let nodes = parser.get_schema_nodes()?;
    let node_texts = nodes
        .into_iter()
        .map(|node| node.text.to_string())
        .collect();
    Ok(ListResult::new(node_texts))
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::bindings::schema::SchemaNode;

    use super::*;

    struct MockParser {
        nodes: Vec<SchemaNode>,
    }
    impl MockParser {
        fn new(nodes: Vec<SchemaNode>) -> MockParser {
            MockParser { nodes }
        }
    }
    impl SchemaParser for MockParser {
        fn get_schema_nodes(&self) -> Result<Vec<SchemaNode>, OpenapiSchemerError> {
            Ok(self.nodes.to_owned())
        }
    }

    #[test]
    fn test_list() -> Result<(), Box<dyn Error>> {
        let parser = MockParser::new(vec![SchemaNode {
            text: "test1".to_string(),
        }]);
        let result = list(parser)?;
        assert_eq!(result, ListResult::new(vec!["test1".to_string()]));
        Ok(())
    }
}
