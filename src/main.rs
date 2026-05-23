use anyhow::Result;
use clap::Parser;

mod commands;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "gitpilot", version, about = "AI-friendly git repository manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Scan(commands::scan::Args),
    Update(commands::update::Args),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan(args) => commands::scan::run(&args),
        Commands::Update(args) => commands::update::run(&args, VERSION),
    }
}
