use std::fmt::Display;

use crate::{
    bindings::{self, OperationParser},
    error::OpenapiSchemerError,
};

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

pub fn list(contents: &str) -> Result<ListResult, OpenapiSchemerError> {
    let parser = bindings::TreeSitterOperationParser::new(contents);
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

    use super::*;
}
