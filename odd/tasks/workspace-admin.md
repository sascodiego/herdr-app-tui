# Workspace admin
Objective: add core-authoritative CRUD for spaces and tabs in herdr-app-tui, with hybrid touch-visible actions and a keyboard palette, matching herdr-app's delegation to herdr-core.
Branch: `workspace-admin` (from `main`). Delivery strategy: ask-on-risk. Test runner: `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, PTY smoke with an isolated core.

## Specs

**S1** — Goal, verbatim: "Ahora necesito la administracion de los spaces, agents, tabs. Los agents los detecta herdr solo asi que no enemos nad que hacer ahi. Los spaces deberiamos poder crearlos al igual que las tabs. Esto tenemos que planificar la estratgia, si vamos por el estilo herdr/tmux (hotkey + ui) o si usamos una estrategia propia"
Agents are out of scope: herdr detects them. Spaces and tabs need full CRUD.

**S2** — Choice and question, verbatim: "hibrido pero crud completo. Supongo que nosotros hablamos con el core y es el core el que se ocuma de msndar las señales correctss d cerrado no?"
Hybrid UI: visible sidebar actions plus a keyboard palette. CRUD means create, read, rename, select/move and close for both spaces and tabs.

**S3** — Core-authoritative rule, verbatim: "La idea es siempre que hagamos uso seguro del core asi como lo haria herdr app delegando las responsabilidades al core, nosotros le pedimos todo al cire no hacenks nada ditecto. ante la duda vemis como es el comportamiento de herdr app oficial"
No direct signals, filesystem writes or terminal manipulation. Only `hc` RPC calls: `terminal.run`, `terminal.kill`, `terminal.list`, `doc.get`, `doc.set`. Match official behavior from herdr-app: CAS `doc.set` with conflict re-read; kill the parent and its child tree, sparing kept children with the `herdr.space` label; no `terminal.rm` in the close path.

**S4** — Authorization, verbatim: "sellado"

**S5** — Key ownership: while the phone sheet is closed, all keys belong to the attached agent. The palette opens only from the panel. No global prefix key.

**S6** — Mobile constraints from `design/system.md`: visible actions live inside the sidebar, respect one-cell gutters and two-cell touch targets; no global bottom bar and no status bar.

**S7** — Verification: RED test per rule before GREEN; one differential check for any shared helper; PTY smoke using an isolated core; full suite and clippy clean. Do not claim a visual or CI check that did not happen.

## Tasks

| ID | S# | Route | Status | Evidence |
| --- | --- | --- | --- | --- |
| T1 | S3,S7 | parent | done | RED missing `apply`, GREEN CAS retry; 202+2 tests, clippy clean; see L6 |
| T2 | S1,S2 | parent | done | RED missing `Mutation`, GREEN CRUD and close-address tests; see L7 |
| T3 | S3 | parent | done | RED missing launch/kill_tree, GREEN core-only lifecycle; 207+2 tests, clippy clean; see L8 |
| T4 | S2,S5,S6 | parent | in_progress | RED input/view test then GREEN sidebar actions, palette and inline prompt |
| T5 | S1,S3 | parent | pending | RED wiring test then GREEN refresh/attach after mutations |
| T6 | S7 | parent | pending | PTY smoke, docs, full suite and clippy |
| T7 | all | parent | pending | work-unit commit(s) recorded here |

## Log

**L1 — 2026-11-21** — "Ahora necesito la administracion de los spaces, agents, tabs. Los agents los detecta herdr solo asi que no enemos nad que hacer ahi. Los spaces deberiamos poder crearlos al igual que las tabs. Esto tenemos que planificar la estratgia, si vamos por el estilo herdr/tmux (hotkey + ui) o si usamos una estrategia propia"

**L2 — 2026-11-21** — "hibrido pero crud completo. Supongo que nosotros hablamos con el core y es el core el que se ocuma de msndar las señales correctss d cerrado no?"

**L3 — 2026-11-21** — "La idea es siempre que hagamos uso seguro del core asi como lo haria herdr app delegando las responsabilidades al core, nosotros le pedimos todo al cire no hacenks nada ditecto. ante la duda vemis como es el comportamiento de herdr app oficial"

**L4 — 2026-11-21** — "sellado"

**L8 — 2026-11-21** — T3 done. Added `launch()` and `kill_tree()` to `workspace.rs`. Launch uses `terminal.run` with `herdr.space`/`herdr.tab` labels, shell argv, cwd and the pane size. Kill walks children through `terminal.list`, spares kept agents with a non-empty `herdr.space` label, calls `terminal.kill` for every doomed address and never calls `terminal.rm`. RED was missing functions; GREEN with two new tests. 207 + 2 tests, clippy clean. Work-unit slice `f7933d1` plus lifecycle changes exceed the 400-line advisory; delivery strategy remains `ask-on-risk` and publishing is not authorized by these local commits.

**L7 — 2026-11-21** — T2 done. Added `workspace::Mutation` with `NewSpace`, `NewTab`, rename, select, move and close variants. `NewSpace` opens with one named tab and terminal; `NewTab` falls back to creating a space when none is active; close returns terminal addresses for the core to end. RED was the missing type; GREEN after implementation, with three new CRUD tests. 205 + 2 tests, clippy clean.

**L6 — 2026-11-21** — T1 done. Added `src/tui/workspace.rs` with a narrow `CoreCalls` trait and `apply()`: it mutates the local model, writes through `doc.set`, and on `error_kind == "conflict"` re-reads with `doc.get`, replays the same change on a fresh `Model` and retries once. Non-conflict errors propagate with context. RED was a compile failure on the missing `apply`; GREEN after implementation. `cargo test --all-targets`: 202 passed + 2 bin tests, 1 ignored, 0 failed; clippy clean.

**L5 — 2026-11-21** — Exploration confirmed official herdr-app behavior: `mirror_sync.rs:94` writes `doc.set` with version and re-reads on `conflict`; `window.rs:2714-2758` creates via `terminal.run` then mutates `Model`; `window.rs:2956-3010` kills the parent plus its child tree, sparing kept children with `herdr.space`; `window.rs:3208-3270` requires confirmation for `space.close`; `window.rs:2051-2068` flushes the captured `Document` to the mirror. No `terminal.rm` in the app's close flow. The TUI will follow that contract.
