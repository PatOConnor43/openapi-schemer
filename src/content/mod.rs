use std::{collections::HashMap, fs::File, io::Read, path::PathBuf, rc::Rc, str::FromStr};

use crate::bindings;

#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub trait ContentProvider {
    fn get(&self, path: PathBuf) -> String;
    fn paths(&self) -> Vec<&PathBuf>;
}

pub struct ContentProviderMap {
    contents: HashMap<PathBuf, String>,
}

impl ContentProviderMap {
    pub fn new() -> Self {
        ContentProviderMap {
            contents: HashMap::new(),
        }
    }

    pub fn from_map(contents: HashMap<PathBuf, String>) -> Self {
        ContentProviderMap { contents }
    }

    pub fn from_open_api_yaml(path: PathBuf) -> Self {
        let mut backing_map: HashMap<PathBuf, String> = HashMap::new();
        let content = get_content_for_path(path.to_owned());
        let refs = bindings::find_refs(&content);
        let external_refs: Vec<String> = refs
            .into_iter()
            .filter(|dollar_ref| !dollar_ref.starts_with("#"))
            .collect();
        backing_map.insert(path.to_owned(), content.to_owned());
        backing_map.insert(PathBuf::from("#"), content.to_owned());
        for reference in external_refs {
            let path = PathBuf::from(reference);
            let content = get_content_for_path(path.to_owned());
            backing_map.insert(path, content);
        }

        ContentProviderMap {
            contents: backing_map,
        }
    }
}

#[cfg_attr(test, mockable)]
fn get_content_for_path(path: PathBuf) -> String {
    let mut content = String::new();
    let mut file = File::open(&path).expect("Unable to open the file");
    // TODO error handling
    file.read_to_string(&mut content)
        .expect("Unable to read the file");
    content
}

impl ContentProvider for ContentProviderMap {
    fn get(&self, path: PathBuf) -> String {
        return match self.contents.get(&path) {
            Some(path) => path.to_string(),
            None => "".to_string(),
        };
    }

    fn paths(&self) -> Vec<&PathBuf> {
        self.contents.keys().collect()
    }
}
impl Into<ContentProviderMap> for HashMap<PathBuf, String> {
    fn into(self) -> ContentProviderMap {
        ContentProviderMap { contents: self }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use mocktopus::mocking::*;

    use crate::{
        bindings,
        content::{ContentProvider, ContentProviderMap},
    };

    #[test]
    fn no_external_refs() {
        let content = r#"
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
        super::get_content_for_path.mock_safe(|_| MockResult::Return(content.to_string()));
        bindings::find_refs.mock_safe(|_| MockResult::Return(vec![]));

        let fully_qualified_path = PathBuf::new();
        let short_root_path = PathBuf::from("#");
        let provider = ContentProviderMap::from_open_api_yaml(fully_qualified_path.to_owned());
        assert_eq!(provider.paths().len(), 2);
        assert!(provider.paths().contains(&&fully_qualified_path));
        assert!(provider.paths().contains(&&short_root_path));
        assert_eq!(provider.get(fully_qualified_path), content);
        assert_eq!(provider.get(short_root_path), content);
    }

    #[test]
    fn external_refs() {
        let root_content = r#"
openapi: '3.0.0'
info:
  version: 1.0.0
  title: Swagger Petstore
  description: Multi-file boilerplate for OpenAPI Specification.
  license:
    name: MIT
  contact:
    name: API Support
    url: http://www.example.com/support
    email: support@example.com
servers:
  - url: http://petstore.swagger.io/v1
tags:
  - name: pets
paths:
  /pets:
    $ref: 'resources/pets.yaml'
  /pets/{petId}:
    $ref: 'resources/pet.yaml'
                "#
        .to_owned();
        let pets_content = r#"
# resources/pets.yaml
          "#
        .to_owned();
        let pet_content = r#"
# resources/pet.yaml
          "#
        .to_owned();
        let root_path = PathBuf::new();
        let short_root_path = PathBuf::from("#");
        let pets_path = PathBuf::from("resources/pets.yaml");
        let pet_path = PathBuf::from("resources/pet.yaml");
        let content_map = HashMap::from([
            (root_path.to_owned(), root_content.to_owned()),
            (pets_path.to_owned(), pets_content.to_owned()),
            (pet_path.to_owned(), pet_content.to_owned()),
        ]);
        super::get_content_for_path.mock_safe(move |path: PathBuf| {
            let s = content_map.get(&path).unwrap();
            MockResult::Return(s.to_owned())
        });

        bindings::find_refs.mock_safe(|_| {
            MockResult::Return(vec![
                "resources/pets.yaml".to_owned(),
                "resources/pet.yaml".to_owned(),
            ])
        });

        let provider = ContentProviderMap::from_open_api_yaml(root_path.to_owned());
        assert_eq!(provider.paths().len(), 4);
        assert!(provider.paths().contains(&&root_path));
        assert!(provider.paths().contains(&&short_root_path));
        assert!(provider.paths().contains(&&pets_path));
        assert!(provider.paths().contains(&&pet_path));
        assert_eq!(provider.get(root_path), root_content);
        assert_eq!(provider.get(pets_path), pets_content);
        assert_eq!(provider.get(pet_path), pet_content);
    }
}
