# Touch-first layout
Objective: redesign the mobile TUI around touch targets and a sidebar that keeps its actions, without degrading the desktop rail.
Branch: `touch-first-layout` (from `main`). Delivery strategy: ask-on-risk. Test runner: `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, plus PTY capture at 40x14, 60x20, 100x30 and 180x50.

## Specs

**S1** — Goal, verbatim: "Bien, seguimos, el yank no me es tan importante como empezar a hacer que sea funcional, mejorar la UI, darle forma"
Yank remains out of scope.

**S2** — Problem, verbatim: "Dijimos que sin footer. Lo que necesito es que haya mejor ux de la tui. En el caso mobile la barra eta muy chica es dificil de accionarla debe tener un poquoto de margen sobre el borde. Es decir toda la view mobile esta muy mal optimozada para los toques touch"
Acceptance: at phone widths the top control is a three-row band with a blank margin row above it and the whole band tappable. The toggle is a fixed-width block spanning all three rows, always hard-right, with two lines of information on the left. Tab rows have touch-sized hit areas. There is no global bottom bar.

**S3** — Correction, verbatim: "La linea de settings keys y reload es valida, porque esta denreo del sidebar. Igjal deberiamos rediseñar el layout oensando en algo mas mobile first UX sin descuidar la cersio desktop"
Acceptance: `settings keys reload detach` stays inside the sidebar; the desktop rail remains left, compact and unchanged in substance.

**S4** — Authorization, verbatim: "sellado"
Scope: shape brief, three artboards under `design/herdr-app-tui-touch/`, one chosen direction implemented in the TUI, visual proof and focused tests.

**S5** — Implementation constraints inherited from `AGENTS.md` and `design/system.md`: shared geometry between rendering and hit-testing, glyph plus word for state, no color-only signal, no `unsafe` outside the documented libghostty FFI, no `unwrap()`/`expect()` in production code, zero clippy warnings.

**S6** — Verification: before/after captures at the four sizes listed in the header, `TestBackend` geometry tests, PTY tap smoke, full tests and clippy. If PNG capture is unavailable locally, report that explicitly and use text captures instead of claiming a visual check that did not happen.

## Tasks

| ID | S# | Route | Status | Evidence |
| --- | --- | --- | --- | --- |
| T1 | S1-S5 | parent, gentle-design | done | 3 artboards + manifest + canvas open; toggle-block correction applied; see L6-L14 |
| T2 | S4 | human choice | blocked | waiting for the user to pick a direction on the canvas; unblocks when the chosen artboard is named |
| T3 | S2,S3,S5 | parent | pending | RED geometry test then GREEN shared geometry and touch targets |
| T4 | S2,S5 | parent | pending | RED state test then GREEN attached/preview/scrolled and panel scroll |
| T5 | S6 | parent | pending | captures, PTY smoke, tests, clippy, docs |
| T6 | all | parent | pending | work-unit commit(s) recorded here |

## Log

**L1 — 2026-11-21** — "Bien, seguimos, el yank no me es tan importante como empezar a hacer que sea funcional, mejorar la UI, darle forma"

**L2 — 2026-11-21** — "Dijimos que sin footer. Lo que necesito es que haya mejor ux de la tui. En el caso mobile la barra eta muy chica es dificil de accionarla debe tener un poquoto de margen sobre el borde. Es decir toda la view mobile esta muy mal optimozada para los toques touch"

**L3 — 2026-11-21** — "La linea de settings keys y reload es valida, porque esta denreo del sidebar. Igjal deberiamos rediseñar el layout oensando en algo mas mobile first UX sin descuidar la cersio desktop"

**L4 — 2026-11-21** — "sellado"

**L5 — 2026-11-21** — Created after the mobile-first redesign proposal. The earlier footer-removal idea was rejected by the user; the sidebar action row is retained. Exploration agent `gentle-ai-explore` failed twice in this session ("no final report"); parent used direct reads and reports that fallback.

**L15 — 2026-11-21** — Work-unit slice `42de8f1 design: give the toggle a fixed three-row block` holds the S2 correction. Running total: 798 authored changed lines in 5 commits on `touch-first-layout`. Publishing is not authorized by these commits.

**L14 — 2026-11-21** — User correction, verbatim: "Sin cambios , lo que te indique de la barra no lo tocaste. Herdr hizo de alguna manera que el boton de toggle este siempr a la derecha y grande. [Image#1] tiene lo que te digo. . son dos o tres filas, el boton ocupa las tres filas con un ancho fijo y siempre alineado a la derecha". The attached phone capture shows the reference: a three-row header, two left info lines, and a fixed-width toggle block spanning all three rows on the right. S2 was rewritten; T1 reopened. Applied to `01-safe-area-sheet.html` only: `.band` with a 10ch `surface` toggle block spanning the full band, left column carrying `○ research`, `tab 1 · 1/2` and `~/herdr · all idle`, no `default` in the header. The parent explained in chat that ratatui cannot optically centre or vary font size, but can paint a fixed-width block across three rows.

**L13 — 2026-11-21** — Work-unit slice `8d709c7 design: show active agents and align the header row` holds the two agent-list/alignment comments. Running total: 785 authored changed lines in 4 commits on `touch-first-layout`. Publishing is not authorized by these commits.

**L12 — 2026-11-21** — Two canvas comments were applied to `01-safe-area-sheet.html` only and resolved: `rkkatgng`, `9l62ffqe`. Verbatim: "Esta seccion debe mostrar los agentes activos. Yo lo que hago es moverme entre agentes a medida que requieren mi atencion, una estadistica no me aporta nada en principio... Herdr detecta los agentes y su estado y los presenta en una lista de facil navegacion" and "Sigue descentrado, desalineado. No me gusta... No se puede poner diferentes sizes de letra... Lo que si necesito es que me lo alinies horizontalmente par que el boton de toggle de la sidebar este siempre a la derecha". Applied: AGENTS is now an attention-ordered navigable list of detected agents with glyph and word state; the header uses a plain two-column row with info left and toggle hard-right, keeping one blank row above and below. The parent will explain the terminal-grid constraint in chat. `gentle-ai-worker` was launched for this bounded slice and also failed with "assistant returned no final report"; parent applied it inline and reports that fallback.

**L11 — 2026-11-21** — Work-unit slice `019dcc0 design: center the sheet header and fix contrast` holds the five resolved AA/header/button comments. Running total: 757 authored changed lines in 3 commits on `touch-first-layout`. Publishing is not authorized by these commits.

**L10 — 2026-11-21** — Five more canvas comments were applied to `01-safe-area-sheet.html` only and resolved: `0wmndgbj`, `bjc4jrmv`, `8pnqj3z9`, `yklw9l3u`, `5f627vnm`. Verbatim feedback included "No me gustan los botones", "Sigue despropocionado el texto no se ajusta en espacio ni tamaño, deberia al menos estar centrado. El \"default\" imagino que es el space de herder-core no lo mostramos aca, vemos si lo mostramos en el sidebar pero en principio lo sacamos" and three AA findings: 2.44:1 on the brief, 2.29:1 on `ubuntu-test` and 2.44:1 on `cargo test --lib`, all against a 4.5:1 target. Applied: plain aligned action labels on the surface band instead of boxes; centered header `research · ~/herdr · 4 tabs ● x` with no `default`; the core space moved to a `SPACES` section in the sidebar; secondary colors switched to the Gotham foreground token, measured at 11.24:1 on bg and 10.52:1 on panel. Canvas refreshed.

**L9 — 2026-11-21** — Work-unit slice `1125cc2 design: apply canvas feedback to the safe-area sheet` holds the six resolved comments. Running total: 685 authored changed lines in 2 commits on `touch-first-layout`. Publishing is not authorized by these commits.

**L8 — 2026-11-21** — Six canvas comments were applied to `01-safe-area-sheet.html` only and resolved: `iu1xt362`, `waf0lkqn`, `nw58xcad`, `5rlxnq41`, `qyeh3j3w`, `vzz1vhf1`. Verbatim feedback included "El close prefiero que sea solo una x", "El desborde lo ponemos en una fila inferior con una tabulacion mayor al del nombre del tab", "no logro darme cuenta d edonde vas a obtener esta informacion que queres mostrar", "cada boton debe tener un box definido", "debe estar alineada abajo incluso cuando se abre y se cierra el teclado debe reacomodarse al alto" and "La paleta de colores la vmos a cambiar, me gustaria que sea mas profundo, mas estilo https://terminalcolors.com/themes/gotham/default/". Applied: open header now shows workspace, path and tab count with a single `x`; overflow moved to a deeper-indented second line; the separator row now reads only data the core actually returns; actions sit on a surface band with boxed buttons; the artboard palette is Gotham Default. Canvas refreshed. **Open decision:** `design/system.md` still pins the Catppuccin palette; syncing it changes the other artboards and the production theme, so that is not included here.

**L7 — 2026-11-21** — Work-unit slice `2f4e1d2 design: explore touch-first mobile layouts` holds T1 and 591 authored changed lines. Delivery strategy `ask-on-risk` is recorded in the header; publishing is not authorized by this commit.

**L6 — 2026-11-21** — T1 done. Wrote `design/herdr-app-tui-touch/01-safe-area-sheet.html`, `02-stacked-deck.html`, `03-rising-peek.html` and `design.json`. All three show the shared collapsed face, the open mobile face and the left desktop rail; the sidebar action row stays in the sheet and rail. Self-check: only the tokens in `design/system.md` appear (page background also changed to `var(--bg)`), one monospace stack, no external URLs, glyph family from the system, focus styles on tappable roles. Canvas opened at http://127.0.0.1:36229/zTNSU3aRHrHrX-n9mTq8B6XEkjThE1cg/d/herdr-app-tui-touch/ . T2 is blocked on the user choosing one direction.
