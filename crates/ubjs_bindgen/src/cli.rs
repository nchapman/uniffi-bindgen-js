use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "uniffi-bindgen-js")]
#[command(about = "Generate JS/TS bindings from UniFFI sources")]
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
    /// Source file: .wasm (primary), .udl, or .dylib/.so/.dll (library mode).
    ///
    /// When a .wasm file is provided, metadata is extracted directly from it
    /// and the file is copied to the output directory.
    /// UDL and library sources always generate FFI-direct TypeScript.
    pub source: PathBuf,
    #[arg(long)]
    pub out_dir: PathBuf,
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// Generate bindings only for this crate (default: first found).
    #[arg(long, name = "crate")]
    pub crate_name: Option<String>,
}
