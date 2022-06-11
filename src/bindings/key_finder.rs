use std::{error::Error, path::PathBuf};

use tree_sitter::{Parser, Query, QueryCursor};

use super::language;
pub struct TreeSitterKeyFinder<T: KeyFinderProvider> {
    path: PathBuf,
    provider: Box<T>,
}

impl<T: KeyFinderProvider> TreeSitterKeyFinder<T> {
    fn get_content(&self, path: PathBuf) -> String {
        self.provider.get(path)
    }
}

pub trait KeyFinder<'a> {
    fn find_children_for_key(&self, key: &'a str) -> Result<Vec<String>, Box<dyn Error>>;
}

impl<'a, T: KeyFinderProvider> KeyFinder<'a> for TreeSitterKeyFinder<T> {
    fn find_children_for_key(
        &self,
        key: &'a str,
    ) -> Result<Vec<String>, Box<(dyn Error + 'static)>> {
        let split_keys: Vec<&str> = key.split('.').collect();
        if split_keys.is_empty() {
            return Err(Box::from("Invalid key"));
        }
        let mut parser = get_parser()?;
        for k in split_keys {
            let query = create_key_query(k);
            let content = {
                let path = &self.path;
                self.get_content(path.to_owned())
            };
            let tree = parser.parse(content.to_string(), None);
            if let None = tree {
                return Err(Box::from("Could not parse file as yaml"));
            }
            let tree = tree.unwrap();
            let query = Query::new(parser.language().unwrap(), &query)?;
            let mut qc = QueryCursor::new();
            let mut found_components = None;
            let bytes = content.as_bytes();
            for qm in qc.matches(&query, tree.root_node(), bytes) {
                println!("{:?}", qm.captures.get(0).unwrap().node.utf8_text(bytes));
                found_components = match qm.captures.get(1) {
                    Some(capture) => Some(capture.node),
                    None => None,
                }
            }
            println!("{:?}", found_components)
        }

        Ok(vec![])
    }
}
fn create_key_query(key: &str) -> String {
    return format!(
        r#"
            (block_mapping_pair key: ((flow_node) @key (eq? @key "{}")) value: (block_node) @value)
            "#,
        key
    );
}
fn get_parser() -> Result<Parser, Box<dyn Error>> {
    let mut parser = Parser::new();
    let language = language();
    parser.set_language(language)?;
    Ok(parser)
}

impl<T> TreeSitterKeyFinder<T>
where
    T: KeyFinderProvider,
{
    pub fn new(path: PathBuf, provider: T) -> Self {
        TreeSitterKeyFinder {
            path,
            provider: Box::new(provider),
        }
    }
}

pub trait KeyFinderProvider {
    fn get(&self, path: PathBuf) -> String;
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, error::Error, path::PathBuf};

    use crate::bindings::key_finder::KeyFinderProvider;

    use super::{KeyFinder, TreeSitterKeyFinder};

    #[test]
    fn find_children_for_key() -> Result<(), Box<dyn Error>> {
        let contents: HashMap<PathBuf, &'static str> = HashMap::from([(
            PathBuf::new(),
            r#"
components:
  schemas:
    Pets:
      type: array
      items:
        $ref: '#/components/schemas/Pet'
        "#,
        )]);

        struct MockKeyFinderProvider {
            responses: HashMap<PathBuf, &'static str>,
        }
        impl KeyFinderProvider for MockKeyFinderProvider {
            fn get(&self, path: PathBuf) -> String {
                if let Some(content) = self.responses.get(&path) {
                    return content.to_string();
                } else {
                    "".to_string()
                }
            }
        }

        let provider = MockKeyFinderProvider {
            responses: contents,
        };
        let finder = TreeSitterKeyFinder::new(PathBuf::new(), provider);
        let children = finder.find_children_for_key("components.schemas");

        Ok(())
    }
}
