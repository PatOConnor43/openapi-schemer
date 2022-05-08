use std::fmt::Display;

use crate::{bindings::OperationParser, error::OpenapiSchemerError};

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

pub fn list<T: OperationParser>(parser: T) -> Result<ListResult, OpenapiSchemerError> {
    let nodes = parser.get_operation_nodes();
    let node_texts = nodes
        .into_iter()
        .map(|node| node.text.to_string())
        .collect();
    Ok(ListResult::new(node_texts))
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::bindings::{self, OperationNode};

    use super::*;

    struct MockParser {
        nodes: Vec<OperationNode>,
    }
    impl MockParser {
        fn new(nodes: Vec<OperationNode>) -> MockParser {
            MockParser { nodes }
        }
    }
    impl OperationParser for MockParser {
        fn get_operation_nodes(&self) -> Vec<bindings::OperationNode> {
            self.nodes.to_owned()
        }
    }

    #[test]
    fn test_list() -> Result<(), Box<dyn Error>> {
        let parser = MockParser::new(vec![OperationNode {
            text: "test1".to_string(),
        }]);
        let result = list(parser)?;
        assert_eq!(result, ListResult::new(vec!["test1".to_string()]));
        Ok(())
    }
}
