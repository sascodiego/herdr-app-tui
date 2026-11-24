# herdr-app-tui — design system

Terminal design system for `herdr-app-tui`. Everything here is expressible in
a terminal cell grid: colors, weight and glyphs. No sizes, radii, shadows or
fonts — a terminal has none of those.

## Sources

| Source | Path | What it gives |
| --- | --- | --- |
| Gotham Default palette | `https://terminalcolors.com/themes/gotham/default/` (Alacritty export) | Canvas, foreground, selection, blue, magenta, cyan, green, yellow and red |
| Herdr app tokens | `../herdr-app/vendor/gpui-kit/crates/shell/src/default-tokens.json` | The desktop app's neutral chrome palette, kept for inherited comparison only |

The palette was changed from Catppuccin to Gotham Default on 2026-11-21 at the
user's request. The terminal exports are the source of the values below; the
`panel` tint is derived as one step above `bg` and `rule` uses Gotham blue.
Gotham's magenta and blue measure under 4.5:1 on this canvas, so the secondary
text roles reuse the foreground token instead of inventing a new midtone.

## Color roles — dark (canvas)

| Token | Value | Usage |
| --- | --- | --- |
| `bg` | `#0c1014` | Terminal canvas and pane background |
| `panel` | `#111820` | Sheet and rail surface, one step up from canvas |
| `surface` | `#0a3749` | Selected row, active tab, fixed toggle block, action band |
| `text` | `#99d1ce` | Primary text: labels, values, agent names |
| `muted` | `#99d1ce` | Secondary text: section headers, counts, paths |
| `dim` | `#99d1ce` | Discreet text: separators, notes, actions at rest |
| `accent` | `#33859e` | Connection/focus marker, active tab glyph |
| `green` | `#2aa889` | Agent state `idle`, connected |
| `yellow` | `#edb443` | Agent state `working`, warning |
| `red` | `#c23127` | Agent state `blocked`, error, disconnected |
| `rule` | `#195466` | Separator line and device border |

Measured contrast: `text`/`muted`/`dim` on `bg` is 11.24:1 and on `panel` is
10.52:1, both above WCAG 2.2 AA's 4.5:1 for small text.

## Color roles — app chrome (inherited, neutral)

| Token | Value | Usage |
| --- | --- | --- |
| `background` | `#0a0a0a` | App-matching neutral canvas, kept as reference |
| `foreground` | `#fafafa` | App-matching primary text, kept as reference |
| `muted_foreground` | `#a3a3a3` | App-matching secondary text, kept as reference |
| `border` | `#262626` | App-matching rule, kept as reference |
| `destructive` | `#f87171` | Destructive label, kept as reference |

The light group exists in the same source and is recorded for completeness
only: the TUI ships dark.

## Typography

A terminal has one type size. Roles come from weight and color:

| Role | Rendering |
| --- | --- |
| Section header | `muted`, bold, with a `▾`/`▸` glyph |
| Row primary | `text`, normal; `bold` only for the selected label |
| Row secondary | `text`, normal, placed on a lower or indented line |
| Discreet text | `text`, normal, identified by position rather than a darker tone |
| Active row | `text` bold on the `surface` band, no marker glyph |
| Action row | `text` on the `surface` band, separated by two spaces |

Because `muted` and `dim` share the foreground value for contrast, hierarchy
comes from boldness, indentation and band color, never from a second size.

## Glyphs

One geometric family, used everywhere. Never mix with emoji.

| Glyph | Meaning |
| --- | --- |
| `○` | Agent idle, unselected |
| `◐` | Agent working |
| `▲` | Agent needs you (blocked) |
| `✓` | Agent done |
| `×` | Error, close |
| `▸` `▾` | Collapsed / expanded section |
| `●` `○` | Core connected / disconnected |
| `›` | Path separator in the header |
| `·` | Secondary separator |

Agent state is always glyph plus word (`◐ working`), never color alone.

**Selection is the `surface` band plus a bold `text` label — never a glyph.**

## Spacing

The cell is the unit.

- The sidebar has **one-cell gutters on top, bottom, left and right**. This is
  the minimum margin the grid allows; never let sidebar text touch an edge.
- One blank row separates groups; never two inside a group.
- Nesting indent is 2, 4 or 6 columns, never more. Overflow detail goes on a
  lower line indented deeper than its tab name.
- Rails truncate with `…`; they never wrap.

## Component rules

- **Header band**: three rows on phone. Row 1 is workspace and tab count,
  row 2 is the current path, row 3 is the aggregate agent total grouped by
  state (`○ idle 1 · ◐ work 1 · ▲ needs 1`). A fixed 8ch toggle block on
  `surface` spans all three rows and is always hard-right (`switch` collapsed,
  `x` open). The core `default` space is not shown here; it lives in a sidebar
  `SPACES` section. The band's aggregate is a summary; the `AGENTS` section
  below stays a navigable list.
- **Rail**: on desktop the panel is a narrow, always-visible rail on the
  **left**, one-cell gutters included, separated from the terminal by a single
  rule. Its header stays compact and one row high.
- **Both faces**: every design of this panel shows the terminal with the band
  and the sheet open. The band geometry is the same in both.
- **Section**: one header row `▾ NAME  count`, then dense rows. A section with
  zero items renders its header and nothing else.
- **AGENTS**: a navigable list of the agents herdr-core detects per workspace,
  ordered by attention (`needs you` before `working` before `idle`). Never
  replace it with a count table.
- **Row**: glyph, label, then the state word, right-aligned. A row never repeats
  a fact already in the header.
- **Item block**: a space, tab or agent owns a contiguous two-row band on
  `panel`, with the name on row one and the detail on row two at the same left
  edge. On the sheet and rail the surrounding surface is `bg`, so the band
  reads as one item without a drawn box. Selection turns the whole band
  `surface` with a bold name; a selected row carries no marker glyph.
- **Nesting step**: a child tab's two-row band starts two cells to the right of
  its parent's band; its name keeps the 2-cell inset and its detail sits at 4.
  Hierarchy comes from that step and the band edge, not from connector lines.
  The accent is never used for hierarchy.
- **Overflow**: horizontal detail truncates with `…`; a path keeps its tail
  (`…/herdr-app-tui`) and a branch keeps its head. Vertical overflow clips only
  between whole blocks: `↑ N more` / `↓ N more` lines in `dim` mark the clipped
  edges. Scrolling moves a whole block and keeps the selected block fully
  visible with one block of context.
- **Actions**: one plain text row at the bottom of the sidebar, on `surface`,
  separated by two spaces and re-laid-out when the on-screen keyboard changes
  the available height. No boxes, buttons or glyph markers.
- **No status bar.** There is no bottom chrome row: connection state lives in
  the header band, keyboard hints live in the terminal emulator's own key bar.

## Self-check

- Colors used by every artboard are drawn from the tables above.
- No artboard uses a font family outside a system monospace stack.
- No artboard declares a size token, radius or shadow: the grid is the unit.
- The phone face shows both the band-only and sheet-open states.
- Every control has a visible focus style and a touch-sized hit area.
