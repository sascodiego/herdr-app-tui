# herdr-app-tui

**A terminal UI client for herdr-core.** Connect to the same core as the
Herdr desktop app, share sessions, workspaces, terminals, and machines
between your phone (Mosh + Termux), your desktop app, and your terminal.

## Why

The Herdr TUI (`herdr`) runs its own server and does not share state with
herdr-app. This project builds a **pure-client TUI** that speaks JSON-RPC
to herdr-core directly — the same backend herdr-app uses — so everything is
shared.

## How it works

```
herdr-app-tui ─── JSON-RPC (framed) ───▶ herdr-core
                                           ▲
herdr-app ───▶ hc ─── JSON-RPC ───────────┘
```

- **No embedded server.** All terminals, panes, sessions, and machines live
  in herdr-core. Detach from the TUI, reattach from the desktop app, and
  vice versa: it is the same state.
- **One Rust binary.** Built with ratatui, crossterm, and tokio. No
  Electron, no Node.js.
- **SSH / Mosh ready.** Run the TUI on a remote machine over Mosh. Your
  agents keep running even if the link drops.

## Getting started

```bash
cargo build --release

# Point it at the same session as herdr-app
export HC_SESSION=my-session
./target/release/herdr-app-tui
```

Requires herdr-core running (started automatically by herdr-app, or
manually with `hc server start`).

## Status

Early development. See `PROJECT.md` for the full specification and task
list.

## License

Apache-2.0# herdr-app-tui
