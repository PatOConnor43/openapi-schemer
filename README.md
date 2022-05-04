# openapi-schemer
A command line tool to query and modify OpenAPI specs

The goal of this project is to give users a way to interact with their OpenAPI
specs in a way that doesn't just feel like a big blob of yaml. Some of the
features I'd like to implement with this tool:
- List all paths/operations/structs in a spec
- Add paths/operations/structs with configurable templates
- Sort paths/operations/structs
- List "related" structs
    - As in, "List all the structs/types used in this operation"

## Usage
```
openapi-schemer

USAGE:
    openapi-schemer <INPUT> <SUBCOMMAND>

ARGS:
    <INPUT>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    help         Print this message or the help of the given subcommand(s)
    operation
```

## Examples
List operations in a spec:
```
cargo run petstore.yaml operation list
/pets
/pets/{petId}
```
