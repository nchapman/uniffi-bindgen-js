pub mod cli;
pub mod js;

use anyhow::Result;

pub fn run(cli: cli::Cli) -> Result<()> {
    match cli.command {
        cli::Command::Generate(args) => js::generate_bindings(&args),
    }
}
