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
cargo build
./target/debug/herdr-app-tui
```

Use `cargo build --release` only when cutting a release; the normal loop is
`cargo check` plus a debug build.

Needs `herdr-core` (and its `hc` CLI) on `PATH`, or the one the desktop app
installed. The session comes from `HC_SESSION`, `default` when unset.

## Running it

```bash
cargo run
```

The screen reacts to events pushed by herdr-core, so typing does not wait for a
terminal list or a document read. A release build is faster for daily use but
is reserved for releases by project policy.

`herdr-app-tui --version` prints the crate version and the short commit hash it
was built from, so you can confirm which code is running.

## Keys

| Key | What it does |
|---|---|
| `ctrl+o` then `o` | Open or close the panel; `tab` stays with the agent |
| `j` `k` / arrows | Move the highlight; the highlighted tab attaches automatically |
| `enter` | Attach to the highlighted tab and give it the keyboard |
| `esc` | Close the panel; quit when nothing is attached |
| `ctrl+q` / `ctrl+\` | Quit (also sends no key to the agent) |
| `ctrl+o` then `c` / `w` / `r` / `x` | New tab / new space / rename tab / close tab; `R` and `X` target the space |
| `alt+1`…`alt+9`, `alt+0` | Select tab 1…9, or tab 10 |
| `alt+↑` / `alt+↓` | Move through the detected agent list |
| `pageup` / `pagedown` | Scroll the frozen history; `q` or `esc` resumes the live pane |

Touch and pointer gestures work by zone:

| Gesture | What it does |
|---|---|
| Tap the band's `switch` / `x` block | Open or close the panel |
| Tap a tab row | Select it, attach, and close the sheet on a phone |
| Wheel or vertical drag in the terminal | Enter copy mode and scroll the history |
| Wheel or vertical drag in the panel | Move the highlight |
| Tap the terminal | Resume the live pane |

`ctrl+o` is a reserved tmux-style prefix. It opens a floating binding panel and
the next key runs the action: `o` toggle sidebar, `c` new tab, `w` new space,
`r` rename tab, `R` rename space, `x` close tab, `X` close space. `esc` cancels before the
action key. There is no typed command palette: `:` remains an ordinary character
and reaches the agent. The same create, rename and close actions are tappable in
the sidebar footer. Rename opens a pre-filled one-line prompt: Enter commits,
Esc cancels. Close asks for `y` or `n`.

Spaces and tabs are written to the shared `herdr-app` document through
`doc.get`/`doc.set` with the core's version check; a conflict re-reads and
replays the change once. Terminals are created with `terminal.run` and closed
with `terminal.kill` through the core, including their child tree while sparing
kept sub-agents. Agents themselves are detected by herdr-core and are not
managed here: only terminals with an OSC 7501 status report appear in `AGENTS`
or its totals. A plain shell is still shown as a tab, but it is not an agent.
Terminals are never removed directly by the TUI.

Copy mode scrolls libghostty's own scrollback, so colors, bold and background
attributes are preserved; no plain-text dump is used.

The TUI asks for mouse reporting directly with `?1000h ?1002h ?1006h` — no
`?1003h` (any-motion) and no `?1015h` (RXVT). That keeps swipes working under
Termux with or without the `mosh-mouse` wrapper, inside tmux, and over mosh;
with the wrapper in front, the rewrite is a no-op. crossterm's
`EnableMouseCapture` emits the two modes that break that path, so we do not use
it.

A tap is a press and release without movement; a drag needs at least two cells
of vertical travel. The terminal emulator must forward pointer events for these
to work; gestures consumed by the emulator never reach the TUI. Clicks are not
sent to the program inside the terminal.

On a phone the top band has three rows: workspace and tab count, current path,
and the agent total grouped by state. A fixed-width toggle block spans all
three rows on the right. The band also names the pane state: `preview`,
`attached` or `scrolled`. The sidebar has a one-cell gutter on every side, and
the `AGENTS` section is a navigable list ordered by who needs attention.

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
