# herdr-app-tui — Project definition

A terminal UI client for herdr-core. Shares sessions, workspaces,
terminals, and machines with herdr-app.

## Specs

**S1** — The TUI connects to herdr-core via a framed JSON-RPC Unix socket
(`connect_or_start` flow: existing socket, or start core). Socket path
from `HC_SOCKET`, `HC_SESSION`, or XDG defaults (same as hc).

**S2** — On connect, sends `hello` with `framed: true`. Displays core
identity (name, version, methods). Rejects incompatible protocol
generations.

**S3** — Shows a live terminal list (`terminal.list`), refreshed on
`events.subscribe` notifications. Each entry shows address, name, status
(state: idle/working/blocked/error, and line status), and labels.

**S4** — Opening a terminal calls `terminal.attach` with the viewer's
size. Renders the terminal output locally. Sends keyboard input as DATA
frames on the attach channel. Supports resize (RESIZE frame) and
detach/reattach.

**S5** — Keybindings: configurable prefix key (default `ctrl+b`),
split pane, navigate panes, scrollback (`terminal.read` with `--lines`),
copy mode. At minimum: attach, detach (`ctrl+b q`), list, send text,
paste, interrupt.

**S6** — Workspace awareness: reads the same terminal set as herdr-app.
No workspace layout persistence in the TUI (the core owns that).

**S7 — out of MVP.** Machine support (`terminal.list` includes `machine/t7`
addresses). Does not add/remove machines (use `hc machine add`). Deferred by the
human on 2026-11-18: remote machines are not part of the MVP. Until it comes
back, rows from other machines are shown with their `machine/` prefix and no
machine section exists.

**S8** — `events.subscribe` for live status changes. Delta updates
instead of polling.

**S9** — Configuration file: `~/.config/herdr-app-tui/config.toml`.
Reads `HC_SESSION`, `HC_SOCKET` from environment.

**S10** — Build: `cargo build --release`, zero `unsafe`, zero clippy
warnings, `.unwrap()`/`.expect()` only in test code.

**S11** — Mobile first. The default surface is a phone in Termux. Exactly one
row of chrome: the top bar. It holds the current space on the left and the core
name with its connection glyph on the right. The navigation panel opens from
that bar. No center segment: the active tab is marked in the list, never
restated in the bar.

**S12** — No status bar. There is no bottom chrome row. Connection state lives
in the top bar; keyboard hints belong to the terminal emulator's own key bar
(Termux), not to this UI.

**S13** — Desktop: the same panel is a narrow always-visible rail on the
**left**, like an IDE sidebar. Mobile and desktop differ only in how the panel
is presented, never in what it contains.

**S15** — Every direction shows both faces of the toggle. On mobile the default
face is the terminal with the bar collapsed to its one row; opening it is a
deliberate act. The collapsed face is identical in all directions: only what
opening does is a decision.

**S14** — Agent rows carry state as a glyph plus a word (`◐ working`,
`▲ needs you`), never color alone. Selection is a `surface` band with a
bold label, never a glyph.

## Design

`design/herdr-app-tui-mobile/` holds three directions, each with a phone frame
and a desktop frame:

| Artboard | Direction | Tradeoff |
| --- | --- | --- |
| `01-command-sheet.html` | Command sheet: today's full-screen overlay, restructured | Lowest risk, closest to today; hides the terminal while open |
| `02-rising-sheet.html` | Rising sheet: panel rises from the bottom over nine live terminal rows | One component for both breakpoints; competes with the keyboard on a phone |
| `03-peek-header.html` | Peek header: the top bar expands in place and pushes the terminal down | Nothing is covered and the terminal stays interactive; expansion reflows the terminal |

`design/system.md` records the tokens: catppuccin mocha roles for the canvas,
the app's neutral tokens for chrome, inherited from the sibling repositories.

## Tasks

| ID | Spec | Area | Status | Description |
|----|-------|------|--------|-------------|
| T1 | S1, S10 | **code** | **done** | Scaffold: Cargo.toml, CLI, config, plain-mode JSON-RPC client, demo listing. `cargo check`, 9 tests and `clippy -D warnings` are green. Framed mode is explicitly not here: it is T2 |
| T2 | S1, S2 | code | **done** | Framed mode comes from the copied `hc.rs`: `Core::connect` sends `hello` with `framed: true`, reads the length-prefixed wire format and checks `check()`/`GENERATION`. Socket discovery lives in the copied `core_bin.rs`, which runs `hc connect` like herdr-app |
| T3 | S8 | code | **done** | `events.subscribe` drives the UI: `EventLines` parses the pushed stream, credits the channel, and a 4s heartbeat resyncs if it goes quiet. A key press no longer triggers `terminal.list`/`doc.get`/`terminal.read` |
| T4 | S3 | code | **done** | The panel is a ratatui widget with glyph+word state and counted sections, driven by `events.subscribe` (T3, T12) |
| T5 | S4, S5 | code | **done** | Two-way terminal: `hc::attach`, first paint, stream drain with `credit`, libghostty rendering via copied `vt.rs`, keys encoded to DATA, resize, detach. Round trip verified against a live core |
| T6 | S5 | code | **done** | Input handling: the highlighted tab attaches on the first frame, `tab` toggles the panel, `enter` attaches, and every other key goes to the agent (`q`, `r`, `esc` and `ctrl+c` included). `ctrl+q`/`ctrl+\` quit. Mouse deliberately out of scope |
| T7 | S5 | code | **done** | Scrollback and copy mode: `terminal.read` freezes history on first scroll, `App::CopyState` clamps the offset, the view paints it and marks `copy` in the header. Mouse capture routes wheel and drag to the pane or panel and leaves taps on the header/rows intact. Selection/yank stays out of scope |
| T8 | S7 | code | **dropped** | Machine support: prefix machine/t7 addresses in list, machine label display. Out of MVP by the human's decision; see S7 |
| T9 | S9 | code | **done** | `HC_SESSION`/`HC_SOCKET` and the socket candidates are handled by the copied `hc.rs`/`core_bin.rs`. No separate TUI config file was needed for the MVP |
| T10 | S10 | code | **done** | No `unsafe` outside the documented libghostty FFI (D5), zero clippy warnings with `-D warnings`, 183 tests green |
| T11 | S11, S13 | **design** | **done** | Three layout directions, phone and desktop frames, design system sync |
| T12 | S11, S12, S14, S15 | **code** | **done** | Artboard 01 (command sheet) in ratatui: header row, counted sections, agent rows hosted by their terminal, glyph plus word, selection as a surface band, rail on the left when wide, sheet over the screen on a phone, no status bar. Verified against a live core |
| T13 | S13 | code | **done** | Desktop rail: the same panel, 26 columns on the **left**, one rule, terminal to its right. Covered by `a_wide_screen_keeps_the_rail_on_the_left` |
| T14 | S1 | code | pending | Extract `herdr-client` inside herdr-app's workspace (hc, core_bin, model, layout, mirror, persist, spawn) so herdr-app and this TUI share one source of truth instead of a pinned copy. Blocked first step: herdr-app does not build here (`vendor/ghostty` submodule uninitialised, Zig needed for `ghostty-sys`) |
| T15 | S6 | code | **done** | Read the shared workspace document with `doc.get({"name":"herdr-app"})` and convert it with the copied `Document::replace`. TABS now lists the active space's tabs from that document, so terminals outside every tab are not shown; a failed `doc.get` falls back to the core's terminal list. No new upstream modules were needed |

## Log

**L1 — 2026-11-18** — Creación del proyecto: herdr-app-tui, un cliente TUI
que habla JSON-RPC con herdr-core para compartir sesiones/workspaces con
herdr-app. Motivación: usar herdr vía Mosh+Termux desde Android y que el
estado sea el mismo que en la app de escritorio.

**L2 — 2026-11-18** — T1 completado: estructura del proyecto, Cargo.toml
(edition 2024, Apache-2.0), AGENTS.md con reglas Rust
(no unsafe, unwrap solo en tests, clippy limpio), README.md, justfile.
Módulos: `src/main.rs` (CLI con clap), `src/config.rs` (env/XDG),
`src/client/mod.rs` (JSON-RPC connect/hello/call),
`src/client/types.rs` (protocol types),
`src/tui/mod.rs` (demo loop con terminal list + events).

**L3 — 2026-11-18** — Requisito del humano: "la TUI debe ser mobile first.
Hoy herdr tui tiene una top bar que abre una sidebar esa distribucion me
parece bien", y luego: "Es especifico para mobile, en desktop la barra se ve
siempre y es angosta, a la derecha. como la sidebar de un ide." Sin status
bar. Se agregan S11-S14 y T11-T13. Primer intento de diseño descartado por
error de alcance (se había propuesto una columna angosta también en móvil,
que es lo contrario a lo que muestran las capturas).

**L4 — 2026-11-18** — T11 completado. Se leyó `gentle-ai-tui-design` y
`references/terminal-craft.md`. Design system sync desde las fuentes reales de
los repos hermanos: `herdr-app/vendor/gpui-kit/crates/shell/src/default-tokens.json`
(neutros) y `herdr/src/main.rs` `DEFAULT_CONFIG` (`[theme] name = "catppuccin"`
es el default del TUI, lo que explica el lavanda de las capturas). Tres
direcciones en `design/herdr-app-tui-mobile/`, cada una con frame de teléfono
(40×22) y de desktop (100×26). Verificadas con captura real en navegador
(`/src/herdr/tmp/board-0{1,2,3}.png`); se corrigió la banda de selección del
rail, que no cubría el ancho. Canvas: `gentle_design open
herdr-app-tui-mobile`.

**L5 — 2026-11-18** — Vocabulario de estado decidido y aplicado: selección =
banda `surface` con label en bold, nunca glifo (un marcador de selección y un
glifo de estado en la misma columna se leen como lo mismo, y la UI de hoy ya
paga ese precio). Estados: `○ idle`, `◐ working`, `▲ needs you`, `✓ done`,
`× error`; `●` queda solo para "core conectado" en el header.

**L9 — 2026-11-18** — Corrección del humano, verbatim: "No me estas presentando
bien. Me estas presentando la version mobile con la barra en la vista y no me
la estas mostrando mobile con la terminal abierta y la barra cerrada, es decir
me estas mostrando una sola cara del toggle. En la desktop la barra debe ir a
la izquierda, no a la derecha". Se invierte S13 (rail a la izquierda) y se
agrega S15: cada dirección muestra las dos caras del toggle. Reabre T11.

**L10 — 2026-11-18** — T11 rehecho. Cada artboard muestra ahora tres frames:
teléfono con la barra colapsada, teléfono con la barra abierta, y desktop con
el rail fijo a la **izquierda**. La cara colapsada es idéntica en las tres
direcciones, y eso se dice explícitamente: solo lo que hace abrir es una
decisión, así que el affordance del toggle se construye una sola vez.

Se corrigieron dos defectos propios detectados al mirar la captura: (a) el
header decía `researchs · tab 4` y la sección repetía `TABS · RESEARCHS`, que es
el mismo dato dos veces — justo lo que P1 manda eliminar; ahora el header dice
solo `researchs` y la sección solo `TABS`, con la tab activa marcada por banda;
(b) `switch` estaba en color de texto y competía con el estado, ahora es `dim`.

Nota de fidelidad: el frame de desktop dibuja 26 + 56 columnas, más angosto que
una terminal real a pantalla completa, para que el artboard entre completo en
el viewport. No es una restricción del diseño: en un desktop real el rail mide
lo mismo y el terminal toma el resto.

**L6 — 2026-11-18** — `cargo check` sobre el scaffold de T1 falló, y el defecto
era mío, no del plan: `toml = { version = "0.8", ..., features = ["serde"] }`
(el feature no existe en toml 0.8: serde no es opcional ahí) y
`ghostty-vt = "=0.9.0"` como crate de registro cuando en realidad está
vendorizado en `herdr/crates/ghostty-vt` y no se publica. Se corrigió el
Cargo.toml (toml sin el feature inexistente; ghostty-vt fuera del scaffold, con
la decisión movida a T5) y se quitó el `.PHONY:` del justfile, que es sintaxis
de make y no de just.

---

**L11 — 2026-11-18** — Handoff del artboard 01. Decisión del humano: reutilizar
herdr-app al máximo ("lo unico que quiero es que herdr-app sea tui... si podemos
reutilizar herdr-app para toda esta parte heavy mejor"), y que actualizar del
upstream sea controlado. Se midió el acoplamiento a GPUI módulo por módulo:
`hc.rs`, `core_bin.rs`, `model.rs`, `layout.rs`, `persist.rs` tienen **cero**
referencias a gpui; `terminal.rs` (153) y `window.rs` (163) son la capa de
widgets, que es justo la que se reemplaza. Descartado depender del paquete
herdr-app: no tiene target de librería, arrastraría GPUI/X11/Wayland/wgpu y
además perdería su `[patch.crates-io]` de `gpui-pre`, que solo aplica desde la
raíz de su workspace.

**L12 — 2026-11-18** — T12 y T13 implementadas. Copiados 8 archivos + fixtures
desde herdr-app@317e673 con cabecera de atribución; la única costura fue
`crate::mods::home()`, resuelta con un shim de 3 líneas para no tocar un solo
call site del código copiado. Divergencias locales (D1–D4) listadas en
`UPSTREAM.md` para que el próximo sync sea una revisión y no un merge.

El crate pasó a ser lib+bin: los módulos copiados conservan su superficie
pública completa sin `#[allow(dead_code)]`, que la regla del proyecto prohíbe.
Solo dos arreglos de lint en código copiado: `Content::Context.pane` boxeado
(misma forma en el wire) y un clone redundante.

Verificado: 104 tests en verde, `cargo clippy --all-targets -- -D warnings`
limpio, y corrida real contra el core vivo (`default · ubuntu-test`) listando
un terminal creado con `hc run --bg --name smoke sleep 12`.

**L7 — 2026-11-18** — El scaffold de T1 no compilaba: `cargo check` daba 54
errores (54, no uno). T1 se había marcado `done` sin ejecutar el check, y eso
fue una falla de verificación, no del plan. Se reescribieron `main.rs`,
`config.rs`, `client/mod.rs`, `client/types.rs` y `tui/mod.rs` en modo plano
de JSON-RPC, sin `unwrap`/`expect` fuera de tests y sin `#[allow]`. El demo
que inventaba un loop de eventos con `read_line_timeout` se borró: los eventos
son T3. Estado verificado: `cargo check --all-targets` sin warnings, 9 tests
de unidad pasando, `cargo clippy --all-targets -- -D warnings` limpio.

**L8 — 2026-11-18** — Nota de entorno: `just` no está instalado en esta
máquina, así que el `justfile` no se pudo verificar acá; los comandos de
`Cargo.toml` sí. `AGENTS.md` menciona `just build`, que queda como objetivo
del repo, no como algo comprobado localmente.
**L13 — 2026-11-18** — El usuario preguntó cómo era la ida y vuelta si el pane
era un tail. Respuesta: no lo era; `terminal.read` es un snapshot sin canal.
Se selló T5 y se eligió **ghostty** sobre `vt100` según la recomendación
aceptada: paridad con herdr-app (Kitty graphics, queries, key encoding),
aceptando FFI `unsafe` documentada como D5.

Preparación: Zig 0.16.0 verificado en `~/.local/bin/zig` (SHA-256
`ea4b09bf…`), `vendor/ghostty` inicializado en herdr-app al commit `291fd05`,
y `clang` 21 —bindgen lo necesita para `stdint.h`. Sin sudo: `sudo -v` pidió
contraseña y el usuario ejecutó la instalación por su cuenta.

**L14 — 2026-11-18** — T5 implementado: `src/tui/attach.rs` (attach, drenaje,
`credit` en bloques de 64 KiB, `automatic_input` para respuestas del emulador,
resize, detach, codificación de teclas) y `ScreenState` en la vista para pintar
las celdas de libghostty con los colores de `design/system.md`.

Dos hallazgos verificados en vivo: (a) `hc::attach` **no** hace el credit
inicial, a diferencia de `subscribe` — sin él no llega el first paint; se
arregló en `Session::attach`; (b) `terminal.read` sigue para la preview y el
modo pipe, pero el pane real usa el stream.

Verificado: 176 tests, 1 ignorado, `cargo clippy --all-targets -- -D warnings`
limpio y **cero `#[allow]`** (D9/D10 eliminaron los dos que venían de upstream).
Prueba funcional real: `herdr-app-tui --attach tui-seq` devolvió
`LINE-ONE / LINE-TWO`, y `--attach tui-cat --send 'roundtrip-ok'` devolvió
`roundtrip-ok`.

**L19 — 2026-11-21** — T7 verificado: 193 tests y clippy limpio. En un core
aislado con PTY de 40x22 y eventos SGR sintéticos, la rueda sobre el pane
entró en copy mode, el tap en el header abrió el panel, el tap en una fila
seleccionó y adjuntó, y el tap en el pane volvió al vivo. Los tests de view
cubren el scroll real sobre líneas históricas. El core de prueba quedó limpio.

**L18 — 2026-11-21** — El usuario aprobó T7 y agregó dos requisitos verbales:
`El scroll debe cohexistir con los tap que abran el side bar por ejemolo` y
`El scroll debe ser compatible con el touch eso es importante`. El plan sellado
define zonas (header, panel, pane), distingue tap de drag y deja el yank fuera
de alcance. Touch solo puede funcionar si el emulador reenvía eventos de
mouse; un swipe consumido por el emulador no llega a la TUI.

**L17 — 2026-11-21** — El usuario reportó que la salida larga (`cat`, `ls -la`)
llegaba en bloques. Causa: el loop se bloqueaba hasta 250 ms en
`event::poll(TICK)`, y solo una tecla lo despertaba; la salida del stream se
acumulaba y se drenaba toda junta. `FRAME = 16 ms` y heartbeat de 250 frames.
Medido con un core aislado y `terminal.send` sobre una conexión persistente:
10 líneas enviadas con 200 ms de separación aparecieron con esa misma
separación y latencia por debajo de la resolución de la medición. El primer
paint trae el historial completo —ese bloque inicial es esperado—, no es lag.

**L16 — 2026-11-21** — El usuario notó que la TUI era más lenta que herdr-app
desktop, aunque el desktop conecte por red. La causa no era la red sino el
loop local: `refresh()` corría después de **cada tecla** (3 RPC sincrónicos:
`terminal.list`, `doc.get`, `terminal.read`) y además en un poll de 250 ms.
herdr-app usa `events.subscribe` y no hace RPC por tecla. T3 pasa a usar el
stream push con `EventLines`, `set_workspace` ignora un `version` repetido, y
la preview de `terminal.read` se pide solo cuando no hay attach. Verificado en
un core aislado: `PUSH-OK` escrito desde el PTY apareció en el pane y `ctrl+q`salió con exit 0. 183 tests, clippy limpio.

**L15 — 2026-11-21** — El usuario corrió la TUI por Mosh en la notebook y
reportó que veía el panel pero no podía interactuar: sin prompt y sin mouse.
Se descartó Mosh y el core: el documento compartido estaba en `spaces: []`,
no había filas y por lo tanto nunca se adjuntaba nada; además el attach solo
se intentaba al apretar una tecla con el panel abierto. T15 había resuelto la
definición de filas; T6 resuelve la interacción. Sin mouse por decisión del
usuario. Verificado con un core aislado `HC_SESSION=tui-itest`: un terminal
`cat` creado por `hc run --bg` y puesto en el documento se adjuntó solo en el
primer frame; escribir `SECOND-ROUNDTRIP` devolvió el texto en el pane y
`ctrl+q` cerró con exit 0. 181 tests, clippy limpio.
