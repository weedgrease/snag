use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "snag", about = "Marketplace listing alerts")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Daemon,
    Check,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => snag::tui::run().await,
        Some(Commands::Daemon) => snag::daemon::run().await,
        Some(Commands::Check) => snag::daemon::check_once().await,
    }
}
