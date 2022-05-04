use clap::{Args, Parser, Subcommand};

mod bindings;
mod operation;
mod path;

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

fn main() {
    let args = Cli::parse();

    println!("{:?}", args);
    match args.input {
        None => unreachable!("Clap requires input"),
        Some(_) => match args.command {
            Commands::Operation(subcommand) => match subcommand.command {
                OperationCommands::List => {
                    operation::list(args.input.unwrap().clone());
                }
            },
            Commands::Path(subcommand) => match subcommand.command {
                PathCommands::List => path::list(args.input.unwrap().clone()),
            },
        },
    }

    ()
}
