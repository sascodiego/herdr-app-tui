# Touch-first layout
Objective: redesign the mobile TUI around touch targets and a sidebar that keeps its actions, without degrading the desktop rail.
Branch: `touch-first-layout` (from `main`). Delivery strategy: ask-on-risk. Test runner: `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, plus PTY capture at 40x14, 60x20, 100x30 and 180x50.

## Specs

**S1** — Goal, verbatim: "Bien, seguimos, el yank no me es tan importante como empezar a hacer que sea funcional, mejorar la UI, darle forma"
Yank remains out of scope.

**S2** — Problem, verbatim: "Dijimos que sin footer. Lo que necesito es que haya mejor ux de la tui. En el caso mobile la barra eta muy chica es dificil de accionarla debe tener un poquoto de margen sobre el borde. Es decir toda la view mobile esta muy mal optimozada para los toques touch"
Acceptance: at phone widths the top control is a three-row band with the whole band tappable. Row 1 carries workspace and tab count, row 2 the path, row 3 the aggregate agent total grouped by state. An 8ch toggle block spans all three rows, always hard-right. The sidebar has a one-cell gutter on top, bottom, left and right. Tab rows have touch-sized hit areas. There is no global bottom bar.

**S3** — Correction, verbatim: "La linea de settings keys y reload es valida, porque esta denreo del sidebar. Igjal deberiamos rediseñar el layout oensando en algo mas mobile first UX sin descuidar la cersio desktop"
Acceptance: `settings keys reload detach` stays inside the sidebar; the desktop rail remains left, compact and unchanged in substance.

**S4** — Authorization, verbatim: "sellado"
Scope: shape brief, three artboards under `design/herdr-app-tui-touch/`, one chosen direction implemented in the TUI, visual proof and focused tests.

**S5** — Implementation constraints inherited from `AGENTS.md` and `design/system.md`: shared geometry between rendering and hit-testing, glyph plus word for state, no color-only signal, no `unsafe` outside the documented libghostty FFI, no `unwrap()`/`expect()` in production code, zero clippy warnings.

**S6** — Verification: before/after captures at the four sizes listed in the header, `TestBackend` geometry tests, PTY tap smoke, full tests and clippy. If PNG capture is unavailable locally, report that explicitly and use text captures instead of claiming a visual check that did not happen.

## Tasks

| ID | S# | Route | Status | Evidence |
| --- | --- | --- | --- | --- |
| T1 | S1-S5, S7, S8 | parent, gentle-design | done | three-row band with distributed info, four-sided gutter; see L16-L18 |
| T2 | S4 | human choice | blocked | waiting for the user to pick a direction on the canvas; unblocks when the chosen artboard is named |
| T3 | S2,S3,S5 | parent | pending | RED geometry test then GREEN shared geometry and touch targets |
| T4 | S2,S5 | parent | pending | RED state test then GREEN attached/preview/scrolled and panel scroll |
| T5 | S6 | parent | pending | captures, PTY smoke, tests, clippy, docs |
| T6 | all | parent | pending | work-unit commit(s) recorded here |
| T7 | S7, S8 | parent | done | `design/system.md` and `design/system.html` synced to Gotham and the new layout rules |

## Specs tail

**S7** — Design system update, verbatim: "Debemos actualizar ese design/system.md si". `design/system.md` (and its system board) describe the Gotham palette, the three-row header band with a fixed toggle block, the four-sided sidebar gutter, the navigable AGENTS list and the plain action row.

**S8** — Band information, verbatim: "A lo que tenemos mas filas podemos distribuir merjor la informacion de la barra. yo pondria en la primera el workspace y las tabs, en la segunda el path y en la tercera el estado de los agentes agregados. Es decir, el total de agentes agrupado por estado". The band rows carry workspace + tabs, path, and grouped agent totals; the sidebar `AGENTS` section remains a navigable list.

## Log

**L17 — 2026-11-21** — Immediate correction, verbatim: "Perdon, dejamos las 3 filas en la banda lo demas todo lo que te dije". The three-row band is restored; the one-row text placement, four-sided sidebar gutter and system sync remain in force. S2 was rewritten again before any artboard edit.

**L19 — 2026-11-21** — Work-unit slice `e6acd9b design: distribute header info and sync the design system` holds T1 and T7. Running total: 1243 authored changed lines in 7 commits on `touch-first-layout`. Publishing is not authorized by these commits.

**L18 — 2026-11-21** — User addition, verbatim: "A lo que tenemos mas filas podemos distribuir merjor la informacion de la barra. yo pondria en la primera el workspace y las tabs, en la segunda el path y en la tercera el estado de los agentes agregados. Es decir, el total de agentes agrupado por estado". Applied to all three artboards and to the system board: row 1 `○ researchs · 4 tabs`, row 2 `~/src/herdr/herdr-app-tui`, row 3 `○ idle 1 · ◐ work 1 · ▲ needs 1`, with the toggle narrowed to 8ch so row 3 fits the 32ch left column. `design/system.md` and `design/system.html` were rewritten to Gotham and the new rules; the earlier design slug's three artboards were also retokenized. No old Catppuccin values remain under `design/`. Canvas refreshed.

**L16 — 2026-11-21** — User correction, verbatim: "Bien, ahora entiendo que 3 filas es mucho, lo que dejaste con el boton es lo que queria. Al ser dos filas la alineacion centrada del texto no lo vas a poder hacer pero no importa dejalo todo en una fila, preferiblemente en la superior. Por otra parte Neceito margen en el sidebar tanto arriba, abajo a la izquierda y la derecha. una columna en cada costado de margen del minimo ancho posible. Debemos actualizar ese design/system.md si". Interpreted and applied: the header band becomes two rows with all text in the upper row and the fixed toggle block spanning both; the sidebar gets one-cell gutters on all four sides; S2 was rewritten, S7 added, T1 reopened and T7 created.

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
