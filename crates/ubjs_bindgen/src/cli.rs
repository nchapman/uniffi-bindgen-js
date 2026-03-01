use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "uniffi-bindgen-js")]
#[command(about = "Generate JS bindings from UniFFI UDL")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Generate(GenerateArgs),
}

#[derive(Debug, Clone, Parser)]
pub struct GenerateArgs {
    pub source: PathBuf,
    #[arg(long)]
    pub out_dir: PathBuf,
    #[arg(long)]
    pub config: Option<PathBuf>,
}
