use std::{error::Error, path::PathBuf};

use tree_sitter::{Node, Parser, Query, QueryCursor, Tree};

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
        // Split the keys
        // For each segment:
        //   - if there isn't a next segment: return the children keys
        //   - if there is a next segment: recurse
        let split_keys: Vec<&str> = key.split('.').collect();
        if split_keys.is_empty() {
            return Err(Box::from("Invalid key"));
        }
        let mut parser = get_parser()?;
        let content = {
            let path = &self.path;
            self.get_content(path.to_owned())
        };
        let tree = parser.parse(content.to_string(), None);
        if let None = tree {
            return Err(Box::from("Could not parse file as yaml"));
        }
        let tree = tree.unwrap();

        let find_node = |root_node: Node<'a>, content: &[u8], key: &str| -> Option<Node<'a>> {
            let query = create_key_query(key);
            let query = Query::new(language(), &query).unwrap();
            let mut qc = QueryCursor::new();
            let mut node: Option<Node<'_>> = None;
            for qm in qc.matches(&query, root_node, content) {
                node = match qm.captures.get(1) {
                    Some(capture) => Some(capture.node),
                    None => None,
                };
                if node.is_some() {
                    return node;
                }
            }
            return node;
        };

        //let starting_tree = match tree {
        //    Some(t) => {
        //        let root = t.root_node();
        //        let n = root.to_owned();
        //        Some(n)
        //    }
        //    None => None,
        //};
        let mut root = Some(tree.root_node());
        let mut last_key = "";
        for k in split_keys {
            root = find_node(root.unwrap(), content.as_bytes(), k);
            last_key = k;
        }
        let root = root.unwrap();
        //let key = root.utf8_text(content.as_bytes()).unwrap();
        //let mut cursor = root.walk();
        //let mut children: Vec<String> = vec![];
        //for child in root.children(&mut cursor) {
        //    let text = child.utf8_text(content.as_bytes());
        //    if let Ok(t) = text {
        //        children.push(t.to_string())
        //    }
        //}

        let query = create_children_query(last_key);
        let query = Query::new(language(), &query).unwrap();
        let mut qc = QueryCursor::new();
        let mut children: Vec<String> = vec![];
        for qm in qc.matches(&query, tree.root_node(), content.as_bytes()) {
            match qm.captures.get(1) {
                Some(capture) => {
                    let text = capture.node.utf8_text(content.as_bytes());
                    if let Ok(t) = text {
                        children.push(t.to_string())
                    }
                }
                None => (),
            };
        }

        println!("{:?}", root.utf8_text(content.as_bytes()));
        println!("{:?}", children);

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
fn create_children_query(key: &str) -> String {
    //(#match? @value "^\w+:")

    return format!(
        r#"
            (block_mapping_pair
             key: ((flow_node) @key (eq? @key "{}"))
             value: (block_node (block_mapping (block_mapping_pair key: (flow_node) @child)
                     )
                 )
             )
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
    Pats:
      type: array
      items:
        $ref: '#/components/schemas/Pat'
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
