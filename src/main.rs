use bindings::{
    operation::TreeSitterOperationParser, path::TreeSitterPathParser,
    schema::TreeSitterSchemaParser,
};
use clap::{Args, Parser, Subcommand};
use content::ContentProviderMap;

mod bindings;
mod content;
mod error;
mod operation;
mod path;
mod schema;

#[derive(Parser, Debug)]
struct Cli {
    #[clap(parse(from_os_str), value_name = "INPUT", value_hint = clap::ValueHint::DirPath, required = true)]
    input: Option<std::path::PathBuf>,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[clap(arg_required_else_help = true)]
    Operation(Operation),
    Path(Path),
    Schema(Schema),
}

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true)]
struct Operation {
    #[clap(subcommand)]
    command: OperationCommands,
}

#[derive(Debug, Subcommand)]
enum OperationCommands {
    /// List the operations for a spec
    List,
}

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true)]
struct Path {
    #[clap(subcommand)]
    command: PathCommands,
}

#[derive(Debug, Subcommand)]
enum PathCommands {
    /// List the paths for a spec
    List,
}

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true)]
struct Schema {
    #[clap(subcommand)]
    command: SchemaCommands,
}

#[derive(Debug, Subcommand)]
enum SchemaCommands {
    /// List the schemas
    List,
}

fn main() {
    let args = Cli::parse();

    match args.input {
        None => unreachable!("Clap requires input"),
        Some(_) => match args.command {
            Commands::Operation(subcommand) => match subcommand.command {
                OperationCommands::List => {
                    let path = ::std::fs::canonicalize(args.input.unwrap()).unwrap();
                    let provider = ContentProviderMap::from_open_api_yaml(path);
                    let parser = TreeSitterOperationParser::new(Box::new(provider));
                    match operation::list(parser) {
                        Ok(result) => println!("{}", result),
                        Err(err) => eprintln!("Failed: {}", err),
                    }
                }
            },
            Commands::Path(subcommand) => match subcommand.command {
                PathCommands::List => {
                    let path = ::std::fs::canonicalize(args.input.unwrap()).unwrap();
                    let provider = ContentProviderMap::from_open_api_yaml(path);
                    let parser = TreeSitterPathParser::new(Box::new(provider));
                    match path::list(parser) {
                        Ok(result) => println!("{}", result),
                        Err(err) => eprintln!("Failed: {}", err),
                    }
                }
            },
            Commands::Schema(subcommand) => match subcommand.command {
                SchemaCommands::List => {
                    let path = ::std::fs::canonicalize(args.input.unwrap()).unwrap();
                    let provider = ContentProviderMap::from_open_api_yaml(path);
                    let parser = TreeSitterSchemaParser::new(Box::new(provider));
                    match schema::list(parser) {
                        Ok(result) => println!("{}", result),
                        Err(err) => eprintln!("Failed: {}", err),
                    }
                }
            },
        },
    }

    ()
}
