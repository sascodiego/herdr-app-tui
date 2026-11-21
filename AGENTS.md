# herdr-app-tui

A ratatui terminal client for
[herdr-core](https://github.com/herdrdev/herdr-core). Shares the same
backend as herdr-app: sessions, workspaces, and terminals live in the core,
visible from both the desktop app and this TUI.

## Rust conventions

- **No `unsafe` code.** Every rule has a safe equivalent. If you believe
  you need `unsafe`, restructure the code instead.
- **Exception — FFI to libghostty.** `vt.rs` and `ghostty-sys` are copied from
  herdr-app and contain `unsafe` only to call libghostty's C ABI. This is the
  one allowed use, documented as D5 in `UPSTREAM.md`. No other module may
  introduce `unsafe`, and no new FFI surface is added without a decision.
- **No `.unwrap()`, `.expect()`, `.get().unwrap()` or similar panicking
  access outside test code.** Production code handles errors explicitly with
  `Result`, `Option`, pattern matching, or fallback logic.
- **Zero clippy warnings.** The build must pass `clippy` cleanly. Do not
  suppress warnings with `#[allow(…)]` or `#[clippy(…)]` attributes; fix
  the code instead. No allowed lints, no exceptions.
- **Edition 2024** (`edition = "2024"` in Cargo.toml).
- **Apache-2.0** license, matching herdr-app and herdr-core.
- **Async with `tokio`.** Use the same async runtime as herdr-core
  (`tokio` with `rt-multi-thread`, `sync`, `time`, `process`, `io-util`,
  `macros`, `signal`).
- **Serde for all protocol types.** JSON-RPC framed communication with
  herdr-core uses `serde` + `serde_json`.
- **Errors as values**, never exceptions. Use `Result<T, E>` and
  `anyhow::Error` at boundary layers.
- **Test coverage on non-trivial logic.** Tests live next to the module
  they cover, in a `#[cfg(test)]` submodule or inline. `.unwrap()` is
  acceptable in test code only.

## Architecture

```
herdr-app-tui  ─── JSON-RPC (framed) ───▶  herdr-core
                                              socket (~/.local/state/herdr-core/…)
```

The TUI is a **pure client**. No server, no PTY management, no persistence:
every terminal, workspace, and machine lives in herdr-core. The TUI
connects, attaches viewers, sends input, and renders terminal output
locally with `ghostty-vt`.

### Module layout

Copied from herdr-app (see `UPSTREAM.md`), do not restyle:

| Module | Responsibility |
|--------|----------------|
| `src/hc.rs` | JSON-RPC over `hc connect`: call, attach stream, events, input, resize, credit |
| `src/core_bin.rs`, `src/core_bin/` | Finds, runs and updates the core binary |
| `src/model.rs` | Spaces, tabs, panes, agents |
| `src/layout.rs` | Layout documents |
| `src/mirror.rs` | Machine mirroring |
| `src/persist.rs`, `src/spawn.rs` | Atomic file writes, process spawning |
| `src/mods.rs` | Local shim: only `home()`, which `core_bin` needs |

Original to this project:

| Module | Responsibility |
|--------|----------------|
| `src/tui/theme.rs` | Colors, from `design/system.md`; no other module names a color |
| `src/tui/glyph.rs` | The glyph family and the state vocabulary (glyph plus word, never color alone) |
| `src/tui/app.rs` | Screen state, pure and testable: rows, cursor, panel, counts |
| `src/tui/view.rs` | The render: header row, panel, terminal area. Rail on the left when wide, sheet on the phone |
| `src/tui/input.rs` | Keys to actions |
| `src/tui/mod.rs` | Connect, refresh, event loop |
| `src/main.rs`, `src/lib.rs` | CLI, and the library target that keeps the copied API public |

## Compatibility

Preserve compatibility with the herdr-core API that ships with released
herdr-app versions. The API schema is in
`herdr-core/crates/herdr-core/src/api/types.rs`.

## How to build and test

Use `cargo check` for the edit-compile loop: it is the fast way to verify that
a change compiles. Build a debug binary only when a run needs one.

```bash
cargo check                          # while writing code
cargo build                          # debug binary, when needed
cargo test
cargo clippy --all-targets -- -D warnings
```

`cargo build --release` is for cutting a release, not for ordinary development.
The same rule applies to `just release`. A dependency that genuinely needs
release-mode verification is evaluated case by case and documented here; none
is known today.

`just build` is the debug build. `just check` runs tests and clippy. Formatting
is not a gate: the copied `crates/ghostty-sys/build.rs` is not rustfmt-clean and
reformatting it would be an upstream divergence.

`just install` depends on `just release` because it copies
`target/release/herdr-app-tui` into `~/.local/bin/` — that is a path dependency,
not a requirement that tests run with `--release`.

## What ships in the box

The binary is standalone: one Rust binary, no Electron. Configuration in
`~/.config/herdr-app-tui/config.toml`. Socket path discovery follows
`HC_SOCKET`, `HC_SESSION`, and XDG conventions (same as herdr-core).

## Contributing

Work on a branch and open a pull request. Commit messages: lowercase
conventional commits, single line, no AI co-author lines.