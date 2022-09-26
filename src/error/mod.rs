use std::fmt;

#[derive(Debug)]
pub enum OpenapiSchemerError {
    OperationList(String),
    PathList(String),
    SchemaList(String),
}

impl std::error::Error for OpenapiSchemerError {}

impl fmt::Display for OpenapiSchemerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OpenapiSchemerError::OperationList(cause) => cause.fmt(f),
            OpenapiSchemerError::PathList(cause) => cause.fmt(f),
            OpenapiSchemerError::SchemaList(cause) => cause.fmt(f),
        }
    }
}
