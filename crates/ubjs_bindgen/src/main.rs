use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = uniffi_bindgen_js::cli::Cli::parse();
    uniffi_bindgen_js::run(cli)
}
