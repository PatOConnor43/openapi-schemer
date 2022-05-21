use std::fmt::Display;

use crate::{bindings::path::PathParser, error::OpenapiSchemerError};

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

pub fn list<T: PathParser>(parser: T) -> Result<ListResult, OpenapiSchemerError> {
    let nodes = parser.get_path_nodes();
    let node_texts = nodes
        .into_iter()
        .map(|node| node.text.to_string())
        .collect();
    Ok(ListResult::new(node_texts))
}
