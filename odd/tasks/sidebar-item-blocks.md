# Sidebar item blocks

Objective: implement `design/sidebar-panels/04-item-blocks.html` in the Rust TUI:
each space, tab and agent is one continuous two-row block; a child tab is shown
by a two-cell band step with no connector lines; overflow clips whole blocks.

Branch: `workspace-admin` (current feature branch). Delivery strategy:
`ask-on-risk`. Test runner: `cargo test --all-targets`,
`cargo clippy --all-targets -- -D warnings`, PTY capture with an isolated core.

## Specs

**S1** — Verbatim: "Me pasa que no me doy cuenta en tu diseño que es un mismo item, que es un hijo y ademas que hay desborde que no se como lo vas a manejar"
The two rows must read as one item, the parent/child link must be visible, and
overflow must have an explicit rule.

**S2** — Verbatim: "No me gustan las rayas que le agregaste para marcar el arbol"
No `│` / `└` connector glyphs. The child is shown by the band's two-cell step.

**S3** — Verbatim: "Alineado al space, sin tabular" and "Pone esto alineado, sin tabular"
Detail rows align with their item's left edge; no extra indentation beyond the
item's own inset.

**S4** — Verbatim: "Son minimo dos filas por item, no una fila del parent y una para los childs."
Every space, tab and agent owns at least two consecutive rows. A space's second
row is its own detail, not the first row of a child.

**S5** — Verbatim: "Dentro de un spaces me interesa que se muestren los tabs."
Tabs are nested inside their space, in their own two-row blocks.

**S6** — Verbatim: "Implement the design artboard design/sidebar-panels/04-item-blocks.html"
Use the artboard as the visual spec, map it to real components, never copy the
artboard HTML. Colors, typography, spacing and decoration come from
`design/system.md`.

**S7** — Verbatim: "Report progress with the gentle_design tool: action \"handoff\", target \"sidebar-panels/04-item-blocks.html\", status \"in_progress\" when you start; status \"implemented\" when it is done, with previewUrl set to the running app's loopback URL (localhost, 127.0.0.1 or [::1]) when one is running, so the canvas can compare it with the artboard."
The TUI has no loopback app URL; report the PTY captures instead and omit
`previewUrl` rather than inventing one.

**S8** — Verbatim: "sellado"
Authorization to implement this feature, including local work-unit commits on
the current feature branch. Push, PR and merge remain user decisions.

## Tasks

| ID | S# | Route | Commit | Status |
| --- | --- | --- | --- | --- |
| T1 | S1,S3,S4 | parent | `153dcb0`, `8987095` | done — two-row blocks, aligned detail and Gotham tokens |
| T2 | S2,S5 | parent | `8987095` | done — stepped child band, no connectors, tabs nested |
| T3 | S1 | parent | `8987095` | done — tail-preserving ellipsis and block-wise clipping |
| T4 | S1,S2,S5 | parent | `8987095` | done — hit-testing, scroll and input over both rows |
| T5 | S6,S7,S8 | parent | `8b41381` | done — docs, differential check, PTY capture, clippy, handoff close |

## Log

**L1** — "Me pasa que no me doy cuenta en tu diseño que es un mismo item, que es un hijo y ademas que hay desborde que no se como lo vas a manejar"

**L2** — "No me gustan las rayas que le agregaste para marcar el arbol"

**L3** — Design comments on `03-attention-first.html`: "Pone esto alineado, sin tabular" and "Alineado al space, sin tabular". Both were applied to that artboard before this feature.

**L4** — Handoff: "Implement the design artboard design/sidebar-panels/04-item-blocks.html", with the instruction to use `design/system.md` as the token source and to report progress through the gentle-design handoff.

**L5** — Plan proposed in chat: five tasks, files `src/tui/view.rs`, `src/tui/mod.rs`, `src/tui/app.rs`/`model.rs`, tests in `view.rs`; risk item 3 because shared render/hit-testing code is touched, so a differential check against `main` is included. Verification with tests, clippy and PTY captures; no HTTP preview URL exists for the TUI.

**L6** — "sellado" — plan sealed; implementation and local work-unit commits authorized on `workspace-admin`.

**L7** — Implementation discovery: `src/tui/theme.rs` still held the old Catppuccin palette while `design/system.md` records Gotham Default. Since the handoff names `design/system.md` as the token source, token values and the theme test are synced inside T1. Risk item 3 still applies: the panel render and hit-testing helpers are shared, so the T5 differential check compares them against `main`.

**L8** — Context backstop: T1-T4 were delegated to one writer with surfaces `src/tui/view.rs`, `src/tui/app.rs`, `src/tui/mod.rs`; the token sync was delegated to a second writer with surface `src/tui/theme.rs`. Both launched together in one turn with disjoint surfaces. The panel worker stopped without a final report; its partial diff was diagnosed inline (`bands` unused, overflow only tested, children keyed as spaces) and the same worker continued with a narrower scope.

**L9 — 2026-11-24** — Feature close. RED: 7 failing item-block tests, then 9 deeper failures at the continuation boundary. GREEN after the fixes: `cargo test --all-targets` 232 lib + 2 bin passed, 0 failed, 1 ignored; `cargo clippy --all-targets -- -D warnings` clean. Real TestBackend buffer checks prove the BG step, `PANEL` block band and `SURFACE` selected band on both rows. `fit_path` preserves the tail with a single `…` and passes a multibyte test. Overflow indicators carry no hit target and clipping happens only between blocks. Risk item 3 differential check: pre-change binary from `f09cc5e` and the new binary were captured in tmux at 80×24 with isolated cores; the only differences are the intended two-row space and tab blocks (plus session/clock text). Phone capture at 40×20 with the sheet open showed `▾ SPACES 1`, `▾ ~`, `/home/dsasco`, `○ tab` and `idle · 1 pane`. No HTTP preview URL exists for the TUI, so the gentle-design handoff closes without `previewUrl`. Work-unit commits: `8b41381` design + doc, `153dcb0` palette, `8987095` code.

**L10 — 2026-11-24** — Work-unit slice boundary: `4ecac54` brings `workspace-admin` to 3299 authored changed lines in 19 commits since `main` (lock files excluded). Delivery strategy remains `ask-on-risk`; publishing is a user decision.
