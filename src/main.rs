//! `herdr-app-tui`: the herdr-app client and model layers, with a ratatui
//! screen in place of the GPUI window.

use clap::Parser;

const HELP: &str = "\
herdr-app-tui — a terminal UI client for herdr-core

Usage:
  herdr-app-tui [options]

Options:
  -h, --help              Show this help
  -V, --version           Show version
      --skill             Print the agent instructions

Keys:
  tab          open or close the panel
  j / k        move the highlight
  r            refresh now
  q            leave the panel, then quit

See herdr.dev — the runtime your coding agents live on.
";

#[derive(Parser)]
#[command(name = "herdr-app-tui", version, about = "terminal UI client for herdr-core")]
struct Cli {
    /// Print the agent instructions and exit.
    #[arg(long, hide = true)]
    skill: bool,
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let cli = Cli::parse();

    if cli.skill {
        println!("{}", include_str!("../AGENTS.md"));
        return 0;
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("HERDR_APP_TUI_LOG")
                .unwrap_or_else(|_| "warn,herdr_app_tui=info".into()),
        )
        .init();

    match herdr_app_tui::tui::run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("herdr-app-tui: {error:#}");
            eprintln!("{HELP}");
            1
        }
    }
}
