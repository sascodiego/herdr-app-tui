//! `herdr-app-tui`: the herdr-app client and model layers, with a ratatui
//! screen in place of the GPUI window.

use clap::Parser;

/// The version clap prints, including the commit behind the build.
const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("HERDR_APP_TUI_GIT_HASH"),
    ")"
);

const HELP: &str = "\
herdr-app-tui — a terminal UI client for herdr-core

Usage:
  herdr-app-tui [options]

Options:
  -h, --help              Show this help
  -V, --version           Show version and commit
      --skill             Print the agent instructions
      --attach TERMINAL   Attach once, print the screen and exit
      --send TEXT         Text to send with --attach

Keys:
  tab          open or close the panel
  enter        attach to the highlighted tab
  j / k        move the highlight
  pageup / pagedown, alt+up / alt+down   scroll; q or esc resumes
  ctrl+q / ctrl+\\   quit

While attached, q, r, esc and ctrl+c belong to the agent.
Touch: tap the header or a row; drag or wheel to scroll.

See herdr.dev — the runtime your coding agents live on.
";

#[derive(Parser)]
#[command(name = "herdr-app-tui", version = VERSION, about = "terminal UI client for herdr-core")]
struct Cli {
    /// Print the agent instructions and exit.
    #[arg(long, hide = true)]
    skill: bool,
    /// Attach once to a terminal, send text, print the screen and exit.
    #[arg(long, value_name = "TERMINAL")]
    attach: Option<String>,
    /// Text to send with --attach.
    #[arg(long, requires = "attach")]
    send: Option<String>,
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

    if let Some(terminal) = cli.attach {
        std::process::exit(herdr_app_tui::tui::attach_once(&terminal, cli.send.as_deref()));
    }

    match herdr_app_tui::tui::run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("herdr-app-tui: {error:#}");
            eprintln!("{HELP}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_names_the_crate_and_its_commit() {
        assert!(VERSION.starts_with(env!("CARGO_PKG_VERSION")));
        assert_eq!(VERSION.matches(" (").count(), 1);
        assert!(VERSION.ends_with(')'));
    }

    #[test]
    fn the_help_lists_the_current_quit_and_scroll_keys() {
        assert!(HELP.contains("ctrl+q / ctrl+\\"));
        assert!(HELP.contains("pageup / pagedown"));
        assert!(!HELP.contains("r            refresh now"));
    }
}
