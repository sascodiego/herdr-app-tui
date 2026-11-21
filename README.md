# herdr-app-tui

**herdr-app, as a terminal UI.** The same client and model layers the Herdr
desktop app uses, with a ratatui screen in place of the GPUI window — so the
app and this TUI share one core: sessions, spaces, tabs and terminals.

## Why

The Herdr TUI (`herdr`) runs its own server. This one does not: it speaks
JSON-RPC to `herdr-core` through `hc connect`, exactly like `herdr-app`, and
renders locally.

## Based on herdr-app

The protocol, core discovery and model are **copied from
[herdr-app](https://github.com/herdrdev/herdr-app)** (Apache-2.0, © Herdr, Inc.),
not reimplemented:

| Copied from `herdr-app/src` | Lines | What it does |
|---|---:|---|
| `hc.rs` | 869 | JSON-RPC over `hc connect`: attach, events, input, resize, flow control |
| `model.rs` | 1129 | Spaces, tabs, panes and agents |
| `layout.rs` | 919 | Layout documents |
| `core_bin.rs` + `core_bin/cli_link.rs` | 549 | Finds and runs the core binary |
| `mirror.rs` | 359 | Machine mirroring |
| `persist.rs` | 119 | Atomic file replacement |
| `spawn.rs` | 17 | Process spawning |

Only the UI layer is original: `src/tui/` (ratatui view, glyph and state
vocabulary, input, theme). The GPUI widget layer of herdr-app is **not** used.

Upstream updates are **deliberate, never automatic**: this copy is pinned to a
herdr-app revision, and every local divergence is listed in
[`UPSTREAM.md`](UPSTREAM.md) so a sync is a review, not a merge.

## Getting started

```bash
cargo build --release
./target/release/herdr-app-tui
```

Needs `herdr-core` (and its `hc` CLI) on `PATH`, or the one the desktop app
installed. The session comes from `HC_SESSION`, `default` when unset.

## Running it

```bash
cargo run --release
```

Use `--release`: libghostty's parser is much slower in a debug build. The
screen reacts to events pushed by herdr-core, so typing does not wait for a
terminal list or a document read.

## Keys

| Key | What it does |
|---|---|
| `tab` | Open or close the panel |
| `j` `k` / arrows | Move the highlight; the highlighted tab attaches automatically |
| `enter` | Attach to the highlighted tab and give it the keyboard |
| `esc` | Close the panel; quit when nothing is attached |
| `ctrl+q` / `ctrl+\` | Quit (also sends no key to the agent) |

The panel lists the tabs of the active space in the shared `herdr-app`
document — the same data the desktop app reads — and uses `terminal.list` only
for their live state. Terminals outside every tab are not shown. If `doc.get`
fails, it falls back to the core's terminal list.

The highlighted tab is attached automatically, including on the first frame:
bytes from herdr-core are rendered by libghostty, and your keys go back to the
program. `tab` opens the panel and takes the keyboard; `esc` gives it back.
While attached, `q`, `r`, `esc` and `ctrl+c` belong to the agent, not to the
screen.

Scripted round trip:

```bash
herdr-app-tui --attach t7 --send 'hello'
```

## Status

Early. `PROJECT.md` holds the specification and the task list;
`design/herdr-app-tui-mobile/` holds the layout directions, and
`01-command-sheet.html` is the one being implemented.

## License

Apache-2.0. Portions copied from herdr-app, Apache-2.0, © Herdr, Inc.
