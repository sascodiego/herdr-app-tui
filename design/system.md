# herdr-app-tui — design system

Terminal design system for `herdr-app-tui`. Everything here is expressible in
a terminal cell grid: colors, weight and glyphs. No sizes, radii, shadows or
fonts — a terminal has none of those.

## Sources

| Source | Path | What it gives |
| --- | --- | --- |
| Aprende Rust design system | `~/dotfiles/designs/aprenderust/DESIGN.md` | Warm charcoal ground, bone-white text, one Rust orange accent, hairline rings |
| herdr web UI | `herdr web ui v0.3.27` screenshot | Two-row badge items, active rail plus warm wash, ghost actions, footer row |
| Claude Code usage dashboard | screenshot, 2026-11-24 | Small uppercase muted section labels, hairline rules, right-aligned metadata |

The palette moved from Catppuccin to this warm system on 2026-11-24 at the
user's request, after three references were supplied. Aprende Rust is the
palette source; the dashboard supplies the label and rule discipline; the herdr
web UI supplies the item structure and the active state. The neutral ramp is
warm stone, never blue-gray.

## Color roles — dark (canvas)

| Token | Value | Usage |
| --- | --- | --- |
| `bg` | `#0d0b0a` | Terminal canvas and pane background |
| `panel` | `#141210` | Rail surface, one step up from canvas |
| `elevated` | `#1a1714` | Phone sheet and header band, two steps up |
| `line` | `#262220` | Hairline separators, the rail rule, panel rules |
| `text` | `#ece6e0` | Primary text: item names, values, emphasis |
| `body` | `#b0a69e` | Second-line detail: paths, state words, metadata |
| `muted` | `#9c928a` | Section labels, counts, caption text |
| `dim` | `#6f665f` | Discreet text: overflow indicators, actions at rest |
| `accent` | `#f74c00` | Panel numerals and focus markers |
| `accent-bright` | `#ff6a1f` | The active item's focus indicator and digit |
| `wash` | `#2a160c` | Active item fill across both rows |
| `rust-line` | `#5c2810` | The active item's one-cell edge |
| `panel-border` | `#332e2a` | A focused panel's rule |
| `green` | `#4ade80` | Agent state `idle`, connected |
| `yellow` | `#f0b429` | Agent state `working`, warning |
| `red` | `#ff6369` | Agent state `blocked`, error, disconnected |

Contrast: `text` on `bg` is near 15:1, `body` near 9:1 and `muted` near 6.5:1,
all above WCAG 2.2 AA's 4.5:1 for small text. `dim` is used only for text that
is not needed to read the screen.

The accent budget is explicit: **orange stays under roughly 10% of the painted
area.** It marks where you are and nothing else. It is never a state colour and
never decoration.

## Typography

A terminal has one type size. Roles come from weight and color:

| Role | Rendering |
| --- | --- |
| Panel header | `NN` numeral in `accent`, title in `muted` uppercase, count right-aligned, then a `─` rule in `line` |
| Item name | `text`, bold when the item is active, otherwise normal |
| Item detail | `body`, normal, on the item's second row |
| State word | the state colour, always paired with its glyph |
| Active item | `text` bold name on `wash`, with a one-cell `rust-line` edge |
| Overflow note | `dim`, normal |
| Panel numeral | `accent` (`01`, `02`, `03`) |
| Action row | `muted`, normal, separated by two spaces |

Hierarchy comes from weight, the uppercase label, the hairline rule and the
accent rail, never from a second size or a full-bleed colour block.

## Glyphs

One geometric family, used everywhere. Never mix with emoji.

| Glyph | Meaning |
| --- | --- |
| `○` | Agent idle, unselected |
| `◐` | Agent working |
| `▲` | Agent needs you (blocked) |
| `✓` | Agent done |
| `●` | Space, a container node |
| `×` | Error, close |
| `▸` `▾` | Collapsed / expanded node |
| `›` | Path separator in the header |
| `·` | Secondary separator |
| `─` | Hairline rule and separator |
| `↑` `↓` | Clipped edges of the list |

Agent and tab state is always glyph plus word (`◐ working`), never color alone.

**Selection is the `wash` fill across both rows plus a one-cell `rust-line`
edge plus a bold `text` name. There is no bright vertical rail and no marker
glyph.**

## Spacing

The cell is the unit.

- The sidebar has **one-cell gutters on top, bottom, left and right**. This is
  the minimum margin the grid allows; never let sidebar text touch an edge.
- One blank row separates items inside a panel; the rule under a panel header
  separates panels.
- Nesting indent is 2, 4 or 6 columns, never more. Overflow detail goes on the
  item's own second row, indented deeper than its name.
- Rails truncate with `…`; they never wrap.

## Component rules

- **Header band**: three rows on phone. Row 1 is workspace and tab count,
  row 2 is the current path, row 3 is the aggregate agent total grouped by
  state (`○ idle 1 · ◐ work 1 · ▲ needs 1`). A fixed 8ch toggle block on
  `elevated` spans all three rows and is always hard-right (`switch` collapsed,
  `x` open). The band's aggregate is a summary; the `01 AGENTS` panel below
  stays a navigable list.
- **Rail**: on desktop the panel is a **34-column**, always-visible rail on the
  **left**, one-cell gutters included, separated from the terminal by a single
  `line` rule. Its header stays compact and one row high.
- **Panels**: three fixed panels, top to bottom: `01 AGENTS`, `02 WORKSPACES`,
  `03 TABS`. Each starts after one blank row with a header row (`NN` numeral in
  `accent-bright`, uppercase `text` title in bold, right-aligned count in `dim`,
  then a `─` rule in `line`) followed by its items. A panel with zero items
  renders its header and nothing else. The active space and the active tab are
  highlighted in different panels, so a panel never shows two competing
  selections.
- **Item block**: a space, tab or agent owns two contiguous rows — a leading
  badge glyph plus the bold name on row one, the detail on row two at the same
  left edge. One blank row separates it from the next item. There is no box.
- **Badge**: one glyph cell before the name. In `WORKSPACES` it is the focus
  indicator (`▸` active, `○` inactive). In `AGENTS` it is the state glyph,
  always with the word on row two.
- **Active item**: a `wash` fill across both rows, a one-cell `rust-line` edge
  at the block's leftmost column, and the name in bold `text`. No bright rail.
  In `WORKSPACES` the active row's focus indicator is `▸` in `accent-bright`;
  in `TABS` it is the `Alt+N` digit in `accent-bright`.
- **Panels are peers**: tabs are not nested inside a space item; `TABS` is its
  own panel for the active space. No nesting indent, no connector lines.
- **AGENTS**: pinned at the top, so the list stays visible while the other two
  panels scroll. It shows every open agent herdr-core reports, from every space
  and tab, ordered by attention (`needs you` before `working` before `idle`).
  Tapping an agent selects its space and tab even when neither was active; the
  same walk works with `Alt+↑/↓`. The selected agent gets the same active
  treatment as a tab: wash fill, `rust-line` edge and bold name. A wheel notch
  over a program that tracks the mouse goes to that program as SGR mouse input
  (buttons 64/65); programs without tracking keep herdr's copy-mode scrollback.
  Never replace the list with a count table.
- **Overflow**: horizontal detail truncates with `…`; a path keeps its tail
  (`…/herdr-app-tui`) and a branch keeps its head. Vertical overflow clips only
  between whole blocks: `↑ N more` / `↓ N more` lines in `dim` mark the clipped
  edges. Scrolling moves a whole block and keeps the selected block fully
  visible with one block of context.
- **Actions**: one plain text row at the bottom of the sidebar, in `muted`,
  separated by two spaces and re-laid-out when the on-screen keyboard changes
  the available height. No boxes, buttons or glyph markers.
- **No status bar.** There is no bottom chrome row: connection state lives in
  the header band, keyboard hints live in the terminal emulator's own key bar.

## Self-check

- Colors used by every artboard are drawn from the tables above.
- No artboard uses a font family outside a system monospace stack.
- No artboard declares a size token, radius or shadow: the grid is the unit.
- The accent never exceeds roughly 10% of the painted area.
- The phone face shows both the band-only and sheet-open states.
- Every control has a visible focus style and a touch-sized hit area.
