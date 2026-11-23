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

**S9** — Sidebar organization, verbatim: "Bien, ahora no aparecn los spaces/sgents/tabs bien organizados en el sidebar, los spaces no aparecen directamente, es decir los workspaces que sera cada grupo de tabs. Solo aparecen tabs y agentes a veces". The sidebar shows a `SPACES` section with every space and its tab count; `TABS` keeps listing only the active space; `AGENTS` remains global and keeps OSC 7501 detection. Tapping a space row selects it in memory only: the core document has no `active` field, so this selection must not be written through `doc.set` or stored in a new file.

**S8** — Agent list, verbatim: "Ahora veo un agent idle en la lista oero yo no tengo ningun agente activo… Creo que nuestra app esta consumiendo los recursos". A plain shell is not an agent, even when the core reports it live. The AGENTS section, its rollup counts and Alt+Up/Down navigation list only terminals with a real OSC 7501 status report, matching herdr-app's published agent list (`window.rs:1957-1964`: `let Some(status) = m.status(addr) else { continue }`; `model.rs:596-605`: no status word means not an agent).

## Tasks

| ID | S# | Route | Status | Evidence |
| --- | --- | --- | --- | --- |
| T1 | S3,S7 | parent | done | RED missing `apply`, GREEN CAS retry; 202+2 tests, clippy clean; see L6 |
| T2 | S1,S2 | parent | done | RED missing `Mutation`, GREEN CRUD and close-address tests; see L7 |
| T3 | S3 | parent | done | RED missing launch/kill_tree, GREEN core-only lifecycle; 207+2 tests, clippy clean; see L8 |
| T4 | S2,S5,S6 | parent | done | RED missing footer actions, GREEN palette key, command parse, prompt state and footer hit-testing; 211+2 tests, clippy clean; see L10 |
| T5 | S1,S3 | parent | done | GREEN by PTY smoke: new tab, close tab and rename; see L11 |
| T6 | S7 | parent | done | PTY smoke, README, 211+2 tests and clippy clean; see L12 |
| T7 | all | parent | done | `bd7165c`, `f7933d1`, `3e296f3`, `2da7b69`, `f8682d5` plus closing commit; see L12 |
| T8 | S8,S7 | parent | done | `0902d55`; RED missing `is_agent`/`agents`; GREEN agent-only list, counts and navigation; 214+2 tests, clippy clean, PTY smoke; see L20 |
| T9 | S9,S3,S7 | parent | done | RED missing `SPACES` section and `panel_space_at`; GREEN rendered spaces, in-memory active selection, global agents unchanged; 216+2 tests, clippy clean, PTY smoke, independent review; see L25 |

## Log

**L1 — 2026-11-21** — "Ahora necesito la administracion de los spaces, agents, tabs. Los agents los detecta herdr solo asi que no enemos nad que hacer ahi. Los spaces deberiamos poder crearlos al igual que las tabs. Esto tenemos que planificar la estratgia, si vamos por el estilo herdr/tmux (hotkey + ui) o si usamos una estrategia propia"

**L2 — 2026-11-21** — "hibrido pero crud completo. Supongo que nosotros hablamos con el core y es el core el que se ocuma de msndar las señales correctss d cerrado no?"

**L3 — 2026-11-21** — "La idea es siempre que hagamos uso seguro del core asi como lo haria herdr app delegando las responsabilidades al core, nosotros le pedimos todo al cire no hacenks nada ditecto. ante la duda vemis como es el comportamiento de herdr app oficial"

**L4 — 2026-11-21** — "sellado"

**L25 — 2026-11-23** — T9 done. `panel_content` now emits `SPACES` before `TABS`, one row per space with a singular/plural tab count; the active row uses the surface band and bold label. `panel_space_at` hit-tests those rows and `select_space` changes `model.active` in memory only, then rebuilds tabs. No `doc.set` and no new persistence file. RED was the missing section/hit-test; GREEN after implementation. 216 + 2 tests passed, 1 ignored, 0 failed; clippy clean. PTY smoke with isolated core `tui-spaces`: before any document, `▾ SPACES 0`; after creating one space, `▾ SPACES 1` with `space 1  1 tab`; after a second, `▾ SPACES 2` with `space 1` and `space 2`. Independent gentle-ai-verify found no defects. The core and tmux session were stopped.

**L24 — 2026-11-23** — Material deviation and reseal. The first sealed plan said a tapped space would be written through `doc.get`/`doc.set` CAS. Evidence contradicted it: `src/mirror.rs:14-17` and herdr-app `src/mirror.rs:10-13` define a document with only `spaces`; the active space lives in `View` and herdr-app persists it to a local `view.json` (`src/window.rs:1201-1211`), not to the core document. User accepted the recommended option, verbatim: "ok aceoto la sugerencia reconendada", then sealed with "sellado". Space selection is in-memory only; no schema change and no new persistence file. T9's CAS expectation is replaced.

**L23 — 2026-11-23** — Seal: user chose "1. ok esta bien la recomendacion" and "2. Global", then "si sellado". Scope: `SPACES` with all spaces and tab counts, `TABS` of the active space, global `AGENTS`, tap-to-select through CAS; no change to core CRUD or agent detection.

**L22 — 2026-11-23** — User correction, verbatim: "Bien, ahora no aparecn los spaces/sgents/tabs bien organizados en el sidebar, los spaces no aparecen directamente, es decir los workspaces que sera cada grupo de tabs. Solo aparecen tabs y agentes a veces". Opened T9 under S9 after confirming the design in `design/system.md:101-107` and artboard 01 already require a `SPACES` section.

**L21 — 2026-11-23** — Work-unit slice boundary: `0902d55` holds T8 (88 inserted, 12 deleted) and brings `workspace-admin` to 1291 authored changed lines in 12 commits since `main`. Independent gentle-ai-verify review found no defects: `App::agents()`, `counts()`, `panel_content()` and `agent_targets()` all use the status-report signal, plain shells stay idle tabs, 214+2 tests pass and clippy is clean. Delivery strategy remains `ask-on-risk`; publishing is a user decision.

**L20 — 2026-11-23** — T8 done. `Terminal::from_value` now keeps the reported status word separate from the lifecycle state: `state: "live"` still renders a shell tab as `○ idle`, but only a status report (`status.state` or a normalized string) sets `is_agent`. `App::agents()` drives `counts`, `panel_view` and `agent_targets`, so the phone band, the sidebar section and Alt+Up/Down agree. RED was the missing field/method; GREEN after implementation. PTY smoke with isolated core `tui-agents2`: one plain shell showed `1 tab`, `○ idle 0 · ◐ work 0 · ▲ needs 0` and no AGENTS section; after starting a fake agent with `state=working:app=pi`, the panel showed `▾ AGENTS 1`, `◐ fakeagent · working`, and the band showed `◐ work 1`. 214 + 2 tests passed, 1 ignored, 0 failed; clippy clean. README updated.

**L19 — 2026-11-22** — User observation, verbatim: "Ahora veo un agent idle en la lista oero yo no tengo ningun agente activo… Creo que nuestra app esta consumiendo los recursos". The prior fix only translated `state: "live"` to the word `idle`; it did not decide whether the terminal is an agent. Opened T8 under S8: derive agent identity from the same core facts herdr-app uses, apply it to `panel_view`, `counts` and `agent_targets`, and add RED/GREEN coverage.

**L18 — 2026-11-22** — Resource investigation: the ~3.36 GB of RSS came from three Pi-launched `rust-analyzer` servers (~1.7 GB in this repo, ~1.1 GB and ~0.5 GB in another project), not from the TUI. A cleanup kill was blocked by a permission-mode change. User decision, verbatim: "No, no vamos a dejar de usar las herramientas que nos estan haciendo tener un gran codigo. Seguimos como se debe usando todo. Yo ire monitoreando los recursos en paralelo. Volvemos a nuestra app". Keep using LSP and all quality tools; do not propose restricting them. Native `mem_session_summary` still fails because the runtime Pi session belongs to project `herdr`; CLI `engram save --project herdr-app-tui` worked after `mem_current_project` verification.

**L17 — 2026-11-21** — User observation, verbatim: "Bien, ahora veo varas cosas uknow y otros hidden ▾ TABS 1 tab · unknown ... ▾ AGENTS 1 · t18 · unknown ... 11 finished hidden ...". The live core returns top-level `state: "live"` and no `status` word for a plain shell; `State::from_word` did not know `live`, so it rendered `· unknown`. `live` now maps to `○ idle` and `finished` to `✓ done`, while real status words keep priority. The `hidden_finished` count is no longer printed in the sidebar: the core already hides those terminals and the row was noise. RED tests then GREEN; 212 + 2 tests, clippy clean.

**L16 — 2026-11-21** — User correction, verbatim: "Cuandl se cierra un agente o una terminal debe ir a otra ta existente y si no existe crear una en ~, hiy al cerrar una tab existiendo otra cierra la tui entera". Applied: `ensure_open_workspace` creates a space/tab with a shell at `mods::home()` when no live tab remains; `ensure_live_selection` follows another live tab when the selected terminal vanished; `close_and_fallback` selects the surviving tab after a close. A closed attached stream now detaches instead of returning an error, and a `terminal.read` race no longer leaves the screen. New terminals are remembered locally until `terminal.list` refreshes, avoiding duplicate fallback shells. PTY smoke: closing the last tab created `t2` with cwd `/home/dsasco`; closing one of two tabs selected the survivor; killing and removing the selected terminal from outside created exactly one new shell at `~` and the TUI stayed alive. 211 + 2 tests, clippy clean.

**L15 — 2026-11-21** — User correction, verbatim: "El tab alterna entre la terminal y el side bar, deberia mandarse a la terminal, lo usamos para autocomoletar en la terminal, no podemos robar funciines escencuales", then: "ctrol+o + o ese es el toggle". Removed `Tab -> TogglePanel`; Tab now maps to `Ignore` and reaches the attached program. Added `Command::TogglePanel` bound to prefix `Ctrl+O` + `o`; the floating panel lists it. PTY smoke: Tab left the sidebar closed, `C-o o` opened it, `C-o o` closed it. 211 + 2 tests, clippy clean. README and HELP updated.

**L14 — 2026-11-21** — User rejected the typed palette flow and its view, verbatim: "El flujo y la view de como se muestra el comando no le gusta, capaz prefieri un oanel flotante o algo mas ux. Vamos por tipo tmux con ctrl+g + Algo. Y tambien lo grsfico ambas cosas esta bien (ctrl+o + c) crear un tab. ctrl+o + w crear un workspace. Alt+num alternsr entre oanes, alt up/down va al agente de abajo en la lista", then confirmed: "CTRL+O es el prefix. los c/w/r/x segun tu propuesta esta perfexto. esc cancela.". Applied: `Action::Prefix` on Ctrl+O with a floating binding overlay; `c` new tab, `w` new space, `r` rename tab, `R` rename space, `x` close tab, `X` close space, Esc cancels. The typed `:` palette and `Command::parse` were removed; `:` reaches bash again. `Alt+1..9`/`Alt+0` select tabs 1..9/10, `Alt+↑/↓` walk the agent list, and scrollback is now PageUp/PageDown only. PTY smoke at 50x18: Ctrl+O rendered the floating panel, Esc dismissed it, Ctrl+O+c created `t2`, and Alt+2 selected the second tab (`tab` shown in the band). README and HELP updated.

**L13 — 2026-11-21** — Post-close UX correction, verbatim: "pero cordate que estoy en un bash, poner : escribe en el promot de bash porque esta bien. No tengl como escribir esos ':new tab'". `:` remains an ordinary character with the panel closed; `Action::Palette` was added for `ctrl+o`, a reserved key like `ctrl+q`, which opens the panel and the command buffer from anywhere. The panel keeps `:` as a shortcut. HELP and README were updated. PTY smoke: with a bash prompt attached and the panel closed, `C-o` opened an empty `:` buffer, typing `new tab` and Enter created `t2` and the panel showed 2 tabs. 212 + 2 tests, clippy clean.

**L12 — 2026-11-21** — Feature close. Work-unit commits: `bd7165c` (CAS write), `f7933d1` (CRUD mutations), `3e296f3` (core lifecycle), `2da7b69` (palette/footer UI), `f8682d5` (command execution). Re-run at close: 211 + 2 tests passed, 1 ignored, 0 failed; clippy clean. 851 authored changed lines across five work units; delivery strategy `ask-on-risk` still applies and publishing is a user decision.

**L11 — 2026-11-21** — T5 done. `handle_key` captures prompt text only while the panel is open; `:` opens the palette, Enter executes, Esc cancels, Backspace edits and `y`/`n` confirms a close. Commands route through `workspace::apply`, `launch` and `kill_tree`; close removes the model first, asks the core to kill the tree, then writes the document. PTY smoke at 40x14 with isolated core `tui-admin`: `:new tab` created `t2` and the panel showed 2 tabs; `:close tab` + `y` killed `t2`, which appeared as `1 finished hidden`, with no `terminal.rm`; `:rename tab` renamed `base` to `baserenamed`. The colon went to the agent while the panel was closed. README documents the palette, footer actions and core-authoritative lifecycle.

**L10 — 2026-11-21** — T4 done. Added `Action::Command` for `:`, `workspace::Command` parsing for the full CRUD set, `App::Prompt` with command/rename/confirm variants, a prompt line in the panel, CRUD action labels in the sidebar footer and `panel_action_at` tap mapping shared with the rendered line. Existing action-row tests were updated. 211 + 2 tests, clippy clean. Execution wiring remains T5.

**L9 — 2026-11-21** — Work-unit slice boundary: `bd7165c`, `f7933d1` and `3e296f3` hold T1-T3 with 452 authored changed lines. Delivery strategy remains `ask-on-risk`; the oversized single-PR choice will be requested only when publishing is asked for.

**L8 — 2026-11-21** — T3 done. Added `launch()` and `kill_tree()` to `workspace.rs`. Launch uses `terminal.run` with `herdr.space`/`herdr.tab` labels, shell argv, cwd and the pane size. Kill walks children through `terminal.list`, spares kept agents with a non-empty `herdr.space` label, calls `terminal.kill` for every doomed address and never calls `terminal.rm`. RED was missing functions; GREEN with two new tests. 207 + 2 tests, clippy clean. Work-unit slice `f7933d1` plus lifecycle changes exceed the 400-line advisory; delivery strategy remains `ask-on-risk` and publishing is not authorized by these local commits.

**L7 — 2026-11-21** — T2 done. Added `workspace::Mutation` with `NewSpace`, `NewTab`, rename, select, move and close variants. `NewSpace` opens with one named tab and terminal; `NewTab` falls back to creating a space when none is active; close returns terminal addresses for the core to end. RED was the missing type; GREEN after implementation, with three new CRUD tests. 205 + 2 tests, clippy clean.

**L6 — 2026-11-21** — T1 done. Added `src/tui/workspace.rs` with a narrow `CoreCalls` trait and `apply()`: it mutates the local model, writes through `doc.set`, and on `error_kind == "conflict"` re-reads with `doc.get`, replays the same change on a fresh `Model` and retries once. Non-conflict errors propagate with context. RED was a compile failure on the missing `apply`; GREEN after implementation. `cargo test --all-targets`: 202 passed + 2 bin tests, 1 ignored, 0 failed; clippy clean.

**L5 — 2026-11-21** — Exploration confirmed official herdr-app behavior: `mirror_sync.rs:94` writes `doc.set` with version and re-reads on `conflict`; `window.rs:2714-2758` creates via `terminal.run` then mutates `Model`; `window.rs:2956-3010` kills the parent plus its child tree, sparing kept children with `herdr.space`; `window.rs:3208-3270` requires confirmation for `space.close`; `window.rs:2051-2068` flushes the captured `Document` to the mirror. No `terminal.rm` in the app's close flow. The TUI will follow that contract.
