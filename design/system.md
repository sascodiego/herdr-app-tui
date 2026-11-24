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
| `line` | `#262220` | Hairline separators, the rail rule, section rules |
| `text` | `#ece6e0` | Primary text: item names, values, emphasis |
| `body` | `#b0a69e` | Second-line detail: paths, state words, metadata |
| `muted` | `#9c928a` | Section labels, counts, caption text |
| `dim` | `#6f665f` | Discreet text: overflow indicators, actions at rest |
| `accent` | `#f74c00` | The active marker only: rail and active name |
| `wash` | `#1f1610` | Active item fill, a warm near-invisible tint |
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
| Section label | `muted`, normal, uppercase, followed by a `─` rule in `line` |
| Item name | `text`, bold when the item is active, otherwise normal |
| Item detail | `body`, normal, on the item's second row |
| State word | the state colour, always paired with its glyph |
| Active item | `text` bold name on `wash`, with a one-cell `accent` rail |
| Overflow note | `dim`, normal |
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

**Selection is the `wash` fill plus the `accent` rail plus a bold `text`
name — never a glyph.**

## Spacing

The cell is the unit.

- The sidebar has **one-cell gutters on top, bottom, left and right**. This is
  the minimum margin the grid allows; never let sidebar text touch an edge.
- One blank row separates items inside a section; the rule under a section
  label separates sections.
- Nesting indent is 2, 4 or 6 columns, never more. Overflow detail goes on the
  item's own second row, indented deeper than its name.
- Rails truncate with `…`; they never wrap.

## Component rules

- **Header band**: three rows on phone. Row 1 is workspace and tab count,
  row 2 is the current path, row 3 is the aggregate agent total grouped by
  state (`○ idle 1 · ◐ work 1 · ▲ needs 1`). A fixed 8ch toggle block on
  `elevated` spans all three rows and is always hard-right (`switch` collapsed,
  `x` open). The band's aggregate is a summary; the `AGENTS` section below
  stays a navigable list.
- **Rail**: on desktop the panel is a **34-column**, always-visible rail on the
  **left**, one-cell gutters included, separated from the terminal by a single
  `line` rule. Its header stays compact and one row high.
- **Section**: one uppercase `muted` label row followed by a `─` rule in `line`.
  A section with zero items renders its label and nothing else.
- **Item block**: a space, tab or agent owns two contiguous rows — a leading
  badge glyph plus the bold name on row one, the detail on row two at the same
  left edge. One blank row separates it from the next item. There is no box.
- **Badge**: one glyph cell before the name. It carries the node type for a
  space (`●`) and the state for a tab or agent, always with the word on row two.
- **Active item**: a one-cell `accent` rail at the block's left edge across both
  rows, a `wash` fill across the rest of both rows, and the name in bold `text`.
- **Nesting step**: a child tab's two-row block starts two cells to the right of
  its parent; its name keeps the 2-cell inset and its detail sits at 4. The
  active rail sits at the block's own step. Hierarchy comes from that step, not
  from connector lines or background bands.
- **AGENTS**: a navigable list of the agents herdr-core detects per workspace,
  ordered by attention (`needs you` before `working` before `idle`). Never
  replace it with a count table.
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
