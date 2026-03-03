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
    /// Deprecated: library mode is now auto-detected from the file extension.
    #[arg(long, hide = true)]
    pub library: bool,
    /// In library mode, generate bindings only for this crate (default: first found).
    #[arg(long, name = "crate")]
    pub crate_name: Option<String>,
}
