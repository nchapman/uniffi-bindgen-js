use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = ubjs_bindgen::cli::Cli::parse();
    ubjs_bindgen::run(cli)
}
