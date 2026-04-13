use std::path::PathBuf;

use clap::{Parser, Subcommand};
use layer_lint::run_check;

#[derive(Parser)]
#[command(name = "layer-lint")]
struct Cli {
    #[arg(long)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Check,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config_path = cli.config.unwrap_or_else(|| PathBuf::from(".layer-lint.yaml"));

    match cli.command {
        Command::Check => run_check(&config_path)?,
    }

    Ok(())
}
