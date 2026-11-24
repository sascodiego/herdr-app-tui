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

**S9** — Verbatim: "No es la pantalla completa es la sidebar esta bien, nos sirve para nuestro caso de uso. Ahora decime si lo que vos me mostraste tiene algo que ver con el ejemplo que te di: https://media.licdn.com/dms/image/v2/D4E22AQHgWl1JSravJg/feedshare-shrink_800/B4EaDQLY4zHoAc-/0/1790199041889?e=2147483647&v=beta&t=OaQm9USPov4NPD_F7QzWsHRats3hsY8nb4WJrW4ywew la idea es que ahora iteres el diseño hasta pulirlo"
Iterate the sidebar against that reference: tabs live inside their space, not
in a separate `TABS` section; section labels are lowercase `spaces` / `agents`;
the space row uses a colour dot plus name with branch/path as its detail; the
selected band covers both rows. Keep two rows per item and no connector lines.

**S10** — Verbatim: "Tambien la distribucion y la calidad de los colores, el contraste y si ves nuestro sidebar esta feo con esos fondos en los active quedan feos", then "prefiero la barra de acento. Con eso sellado"
Aesthetic pass only, with structure frozen: wider desktop rail, one blank row
between items, tenuous divider between sections, bold names and readable detail
contrast using existing tokens, state-coloured dots, and a one-cell accent bar
as the active marker instead of the full `SURFACE` background band.

**S11** — Verbatim: "No me gusta, le falta color, le falta ux, no esta nada lindo, no se como hacer para que entiendas como lo quiero, lo quiero minimalista, elegante, con colores agradables sin saturar ni chillon, no se que mas darte de ejemplo"
Minimalist and elegant, with pleasant colour that never saturates or shouts.

**S12** — Verbatim: "Capaz podemos usar @~/dotfiles/designs/aprenderust/DESIGN.md utiliza las skilles de stitch para entender el archivo"
Adopt the Aprende Rust design system as the palette source: warm charcoal
ground, bone-white text, one Rust orange accent kept under ~10% of the surface,
hairline separators, dense chrome.

**S13** — Verbatim: "Quiero ademas elegancia, moderno como claude code moderno... esto no es una tui pero refleja mi estilo y contraste"
Add the Claude Code dashboard discipline: small uppercase muted section labels,
hairline rules, right-aligned metadata, subtle fills and no hard borders.

**S14** — Verbatim: "Mira esta referencia tambien, no es tui pero hay una distribucion de paneles con un sctive lindo"
Adopt the herdr web UI item structure: two-row item with an icon badge, active
state as a left amber rail plus a subtle warm wash across both rows, ghost
actions and a footer action row.

**S15** — Verbatim: "sellado"
Sealed scope for T8: apply the merged warm theme to the whole TUI, not only the
sidebar.

**S16** — Verbatim: "Esta mejor pero sigue feo. vamos a sacar el active bar ese que es horrible. Vamos a separar el space activo y la tab activa en paneles diferentes"
Remove the accent rail entirely, and separate the active space and the active
tab into different panels so each panel highlights one thing only.

**S17** — Verbatim: "Bueno pero no una fila por item activable no porque somos mobile first. y eso nos da mas posibilidad de agregar infirmacion contextual"
Keep two rows per item, because the TUI is mobile-first and the second row is
where contextual information goes.

**S18** — Verbatim: "sellado"
Sealed scope for T9. Three panels with numbered headers (`01 WORKSPACES`,
`02 TABS`, `03 AGENTS`), two-row items, active treatment as a warm wash plus a
thin `rust-line` border with no vertical rail, and only data the core reports:
repo branch, state word, `app`, elapsed time and pane count. Fields the core does
not expose (git diff, view type, cost or tokens, free-text phase, sidebar search)
stay out. No new panel-focus keys in this pass; the tab panel is labelled
`TABS`.

## Tasks

| ID | S# | Route | Commit | Status |
| --- | --- | --- | --- | --- |
| T1 | S1,S3,S4 | parent | `153dcb0`, `8987095` | done — two-row blocks, aligned detail and Gotham tokens |
| T2 | S2,S5 | parent | `8987095` | done — stepped child band, no connectors, tabs nested |
| T3 | S1 | parent | `8987095` | done — tail-preserving ellipsis and block-wise clipping |
| T4 | S1,S2,S5 | parent | `8987095` | done — hit-testing, scroll and input over both rows |
| T5 | S6,S7,S8 | parent | `8b41381` | done — docs, differential check, PTY capture, clippy, handoff close |
| T6 | S9 | parent | pending | done — three-round visual polish against the reference |
| T7 | S10 | parent | pending | done — aesthetic pass: distribution, contrast, accent-bar selection |
| T8 | S11,S12,S13,S14,S15 | parent | pending | done — warm theme, uppercase sections, badge items, rail+wash active |
| T9 | S16,S17 | parent | pending | done — three panels, numbered headers, no rail, wash+border active |

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

**L10 — 2026-11-24** — Work-unit slice boundary: the closing commits bring `workspace-admin` to 4452 authored changed lines in 29 commits since `main` (lock files excluded). Delivery strategy remains `ask-on-risk`; publishing is a user decision.

**L11 — 2026-11-24** — User correction, verbatim: "Esta aceptable, el theme esta espantoso no se por que lo tocaste pero quedo horrible", then "reverti el theme". The T1 palette sync was an overreach: `design/system.md` records Gotham, so the code was globally retheme without asking. Reverted `src/tui/theme.rs` to the Catppuccin values at `f09cc5e`, including the module doc and palette test. The two-row block layout is unchanged. `design/system.md` and `design/sidebar-panels/` deliberately keep Gotham tokens by the user's choice; the code/docs palette mismatch is known and out of scope until decided. Re-check: 232 + 2 tests pass, 1 ignored; clippy clean. Commits: `c666cc4` theme revert, `b0905f0` doc record. `design/sidebar-panels/handoffs.json` records the artboard as implemented.

**L12 — 2026-11-24** — User confirmed the tmux-to-PNG sidebar capture is useful: "No es la pantalla completa es la sidebar esta bien, nos sirve para nuestro caso de uso." Added the LinkedIn reference image as S9 and asked for three polish rounds. Comparison found one real gap: the current code still renders `TABS` as a separate section, so tabs are not nested inside their space.

**L13 — 2026-11-24** — "sellado" — scope for T6: three rounds, capture after each, no palette or shortcut changes. Round 1 nests tabs under the active space and removes the `TABS` section; round 2 matches the reference's lowercase section labels and dot/detail space row; round 3 tunes vertical rhythm and compares the final PNG.

**L14 — 2026-11-24** — T6 done. RED: 9 failures for the old `▾ SPACES`/`▾ TABS` strings, separate tab section, dot/detail format and selected-band step. GREEN: 234 lib + 2 bin tests pass, 1 ignored; clippy clean. Rounds 1-2 removed the separate `TABS` section, nested the active space's tabs as two-row child blocks, lowercased `spaces`/`agents`, replaced the disclosure glyph with a dot, and moved the tab count into the space detail. Round 3 compared tmux-to-PNG captures at 80x24 and 40x20 with a second space and a fake OSC 7501 agent: `spaces` with two two-row items, the selected space's tab nested below it, and `agents` with `working · pi`. Kept the two-row rule, the two-cell step, no connector lines and the existing Catppuccin palette. No further geometry change was needed.

**L15 — 2026-11-24** — User correction, verbatim: "Esta feo, no es la estructura la que tenias que modificar es la estetica", then "Tambien la distribucion y la calidad de los colores, el contraste y si ves nuestro sidebar esta feo con esos fondos en los active quedan feos". Opened T7 under S10. Proposed five aesthetic-only changes and asked which active marker to use; user answered "prefiero la barra de acento. Con eso sellado". Structure and shortcuts are frozen; no global palette change.

**L16 — 2026-11-24** — T7 done. RED: 8 failures for the 26-column rail, missing blank rows and divider, full-row `SURFACE` background, non-bold inactive names and a muted inactive-space dot. GREEN: 241 lib + 2 bin tests pass, 1 ignored; clippy clean. The rail is now 34 columns, items are separated by one blank row, a `dim` divider separates `spaces` from `agents`, names are bold `TEXT`, details are normal `TEXT`, headers are bold `DIM`, and the active item uses a one-cell `ACCENT` bar instead of any full-row background. `design/system.md` was updated to match. Final desktop and phone PNGs were captured with two spaces and a `working · pi` agent and sent by Telegram with HTTP 200.

**L17 — 2026-11-24** — User rejected the T7 result: "No me gusta, le falta color, le falta ux, no esta nada lindo...". Three references were provided after that. `~/dotfiles/designs/aprenderust/DESIGN.md` (read with the Stitch skills) supplies the palette; a Claude Code usage dashboard supplies the section-label/rule discipline; the herdr web UI screenshot supplies the two-row badge item and the rail-plus-wash active state. T8 was opened for the merged direction, scoped to the whole TUI.

**L18 — 2026-11-24** — "sellado" — T8 authorized. Tokens: `bg #0d0b0a`, `panel #141210`, `elevated #1a1714`, `line #262220`, `text #ece6e0`, `body #b0a69e`, `muted #9c928a`, `accent #f74c00`, active wash `#1f1610`, `green #4ade80`, `yellow #f0b429`, `red #ff6369`. Sidebar sections become uppercase muted labels with a hairline rule, items keep two rows with a leading badge glyph, and the active item gains the warm wash plus the rust rail. Palette stays under the accent budget; no new fonts or decoration.

**L20 — 2026-11-24** — T9 done. RED: 9 failures for the wash value, the three numbered headers, tab digit/glyph, tab and space details, the rust edge/wash, agent elapsed and flat tap routing. GREEN: 253 lib + 2 bin tests pass, 1 ignored; clippy clean. Sidebar is now `01 WORKSPACES` / `02 TABS` / `03 AGENTS`, each a numbered header with a `line` rule and right-aligned count; items keep two rows with a blank row between them; the active space and the active tab are highlighted in different panels with `wash` plus a one-cell `rust_line` edge and no accent rail. Headers verified in the raw ANSI: numeral `#f74c00`, title and count `#9c928a`, rule `#262220`. Desktop and phone PNGs sent by Telegram with HTTP 200.

**L21 — 2026-11-24** — Blocker found on the branch field. `folder_info` is populated in herdr-app by running git itself (`herdr-app/src/window.rs:2543`, `folder_info()` at `:5283` spawns git in a background executor), not by herdr-core: a grep of `herdr-core/src/api` finds no branch or folder field. So the TUI cannot get a repo branch from the core, and today the second row falls back to the space folder path. Getting the branch would require the TUI to run git like herdr-app does, which is a departure from the core-authoritative rule in S3. Decision pending with the user; the capture reflects the honest fallback.

**L19 — 2026-11-24** — T8 done. RED: 18 renderer failures after the tokens landed. GREEN: 244 lib + 2 bin tests pass, 1 ignored; clippy clean. `theme.rs` now carries the warm tokens plus `BODY`, `WASH`, `ELEVATED`, `LINE`, with `SURFACE` kept only as a `WASH` alias and no renderer painting a hard band. Sections are `SPACES` / `AGENTS` uppercase `muted` with a `line` hairline; items are badge plus bold-when-active name and a `body` detail; the active item has the one-cell `accent` rail plus the `wash` fill across both rows. `design/system.md` and `design/system.html` were retokenized to match. Desktop 80x24 and phone 40x20 PNGs were captured with two spaces and a `working · pi` agent and sent by Telegram with HTTP 200. The section label was corrected from `WORKSPACE` to `SPACES` before the capture.
