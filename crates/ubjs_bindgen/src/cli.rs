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
    /// Source file: .wasm (FFI-direct), .udl, or .dylib/.so/.dll (library mode).
    ///
    /// When a .wasm file is provided, metadata is extracted directly from it
    /// and FFI-direct TypeScript is generated automatically.
    pub source: PathBuf,
    #[arg(long)]
    pub out_dir: PathBuf,
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// Path to a compiled .wasm file (for UDL or library-mode sources).
    /// Enables FFI-direct output; the file is copied to the output directory.
    /// Not needed when source is already a .wasm file.
    #[arg(long)]
    pub wasm: Option<PathBuf>,
    /// Deprecated: library mode is now auto-detected from the file extension.
    #[arg(long, hide = true)]
    pub library: bool,
    /// Generate bindings only for this crate (default: first found).
    #[arg(long, name = "crate")]
    pub crate_name: Option<String>,
}
