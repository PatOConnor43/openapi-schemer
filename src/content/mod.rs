use std::{collections::HashMap, fs::File, io::Read, path::PathBuf};

use crate::bindings;

#[cfg(test)]
use mocktopus::macros::mockable;

#[cfg_attr(test, mockable)]
pub trait ContentProvider {
    fn get_content(&self, path: PathBuf) -> String;
    fn paths(&self) -> Vec<&PathBuf>;
}

#[cfg_attr(test, mockable)]
pub struct ContentProviderMap {
    contents: HashMap<PathBuf, String>,
    root_file: PathBuf,
}

#[cfg_attr(test, mockable)]
impl ContentProviderMap {
    pub fn new() -> Self {
        ContentProviderMap {
            contents: HashMap::new(),
            root_file: PathBuf::from("#"),
        }
    }

    pub fn from_map(contents: HashMap<PathBuf, String>) -> Self {
        ContentProviderMap {
            contents,
            root_file: PathBuf::from("#"),
        }
    }

    pub fn from_open_api_yaml(path: PathBuf) -> Self {
        let mut backing_map: HashMap<PathBuf, String> = HashMap::new();
        let working_directory = path.parent().unwrap();
        let content = get_content_for_path(path.to_owned());
        let refs = bindings::find_refs(&content);
        let external_refs: Vec<String> = refs
            .into_iter()
            .filter(|dollar_ref| !dollar_ref.starts_with("#"))
            .collect();
        backing_map.insert(path.to_owned(), content.to_owned());
        backing_map.insert(PathBuf::from("#"), content.to_owned());
        for reference in external_refs {
            let mut path = PathBuf::new();
            path.push(working_directory);
            path.push(reference);
            let path = canonicalize(path);
            let path = path.unwrap();
            let content = get_content_for_path(path.to_owned());
            backing_map.insert(path, content);
        }

        ContentProviderMap {
            contents: backing_map,
            root_file: path,
        }
    }
}

#[cfg_attr(test, mockable)]
fn get_content_for_path(path: PathBuf) -> String {
    let mut content = String::new();
    let mut file = File::open(&path).unwrap();
    // TODO error handling
    file.read_to_string(&mut content)
        .expect("Unable to read the file");
    content
}

#[cfg_attr(test, mockable)]
fn canonicalize(path: PathBuf) -> Result<PathBuf, ::std::io::Error> {
    ::std::fs::canonicalize(path)
}

#[cfg_attr(test, mockable)]
impl ContentProvider for ContentProviderMap {
    fn get_content(&self, path: PathBuf) -> String {
        if path == PathBuf::from("#") {
            return self.contents.get(&self.root_file).unwrap().to_owned();
        }

        let root_directory = self.root_file.parent().unwrap();
        let mut full_path = PathBuf::new();
        full_path.push(root_directory);
        full_path.push(path);
        let full_path = canonicalize(full_path).unwrap();
        return self.contents.get(&full_path).unwrap().to_owned();
    }

    fn paths(&self) -> Vec<&PathBuf> {
        self.contents.keys().collect()
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
        super::canonicalize.mock_safe(move |path: PathBuf| MockResult::Return(Ok(path)));
        bindings::find_refs.mock_safe(|_| MockResult::Return(vec![]));

        let fully_qualified_path = PathBuf::from("/test/test.yaml");
        let short_root_path = PathBuf::from("#");
        let provider = ContentProviderMap::from_open_api_yaml(fully_qualified_path.to_owned());
        assert_eq!(provider.paths().len(), 2);
        assert!(provider.paths().contains(&&fully_qualified_path));
        assert!(provider.paths().contains(&&short_root_path));
        assert_eq!(provider.get_content(fully_qualified_path), content);
        assert_eq!(provider.get_content(short_root_path), content);
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
        let root_path = PathBuf::from("/test/test.yaml");
        let short_root_path = PathBuf::from("#");
        let pets_path = PathBuf::from("/test/resources/pets.yaml");
        let pet_path = PathBuf::from("/test/resources/pet.yaml");
        let content_map = HashMap::from([
            (root_path.to_owned(), root_content.to_owned()),
            (pets_path.to_owned(), pets_content.to_owned()),
            (pet_path.to_owned(), pet_content.to_owned()),
        ]);
        super::get_content_for_path.mock_safe(move |path: PathBuf| {
            let s = content_map.get(&path).unwrap();
            MockResult::Return(s.to_owned())
        });

        super::canonicalize.mock_safe(move |path: PathBuf| MockResult::Return(Ok(path)));

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
        assert_eq!(provider.get_content(root_path), root_content);
        assert_eq!(provider.get_content(pets_path), pets_content);
        assert_eq!(provider.get_content(pet_path), pet_content);
    }
}
