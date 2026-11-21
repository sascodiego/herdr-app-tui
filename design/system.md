# herdr-app-tui — design system

Terminal design system for `herdr-app-tui`. Everything here is expressible in
a terminal cell grid: colors, weight and glyphs. No sizes, radii, shadows or
fonts — a terminal has none of those.

## Sources

| Source | Path | What it gives |
| --- | --- | --- |
| Herdr app tokens | `../herdr-app/vendor/gpui-kit/crates/shell/src/default-tokens.json` | The desktop app's neutral chrome palette (light and dark) |
| Herdr TUI default theme | `../herdr/src/main.rs` (`DEFAULT_CONFIG`, `[theme] name = "catppuccin"`) | The TUI's default built-in theme name: `catppuccin` |

This repository declares no design sources of its own: `gentle_design list`
reports `Design sources: none detected` for `herdr-app-tui`. The two rows above
are read from the sibling repositories and recorded here as **inherited**
tokens, not as local project tokens.

The lavender selection band visible in the mobile captures is consistent with
Catppuccin Mocha `surface0` on `lavender`. That is an observation from the
captures plus the default theme name, not a value read from a token file.

## Color roles — dark (canvas)

The TUI runs dark. Values follow Catppuccin Mocha, the theme herdr's TUI uses
by default.

| Token | Value | Usage |
| --- | --- | --- |
| `bg` | `#11111b` | Terminal canvas and pane background |
| `panel` | `#181825` | Sheet and rail surface, one step up from canvas |
| `surface` | `#313244` | Selected row, active tab row, hover on a row |
| `text` | `#cdd6f4` | Primary text: labels, values, agent names |
| `muted` | `#7f849c` | Secondary text: section headers, counts, paths |
| `dim` | `#585b70` | Disabled text, separators, footer actions at rest |
| `accent` | `#b4befe` | Selection marker, focus ring cell, active tab glyph |
| `green` | `#a6e3a1` | Agent state `idle`, connected |
| `yellow` | `#f9e2af` | Agent state `working`, warning |
| `red` | `#f38ba8` | Agent state `blocked`, error, disconnected |

## Color roles — app chrome (inherited, neutral)

From the app's token file, dark group. Used only where the TUI must read as the
same product as the desktop app: the window header when a core is named, and
disabled affordances.

| Token | Value | Usage |
| --- | --- | --- |
| `background` | `#0a0a0a` | App-matching neutral canvas (alternative to `bg`) |
| `foreground` | `#fafafa` | App-matching primary text |
| `muted_foreground` | `#a3a3a3` | App-matching secondary text |
| `border` | `#262626` | App-matching rule and separator |
| `destructive` | `#f87171` | Destructive action label |

Light group exists in the same source (`background` `#ffffff`, `foreground`
`#0a0a0a`, `border` `#e5e5e5`) and is recorded for completeness only: the TUI
ships dark.

## Typography

A terminal has one type size. Roles come from weight and color:

| Role | Rendering |
| --- | --- |
| Section header | `muted`, bold, with a `▾`/`▸` glyph |
| Row primary | `text`, normal |
| Row secondary | `muted`, normal |
| Row disabled | `dim`, normal |
| Active row | `text` (bold label) on the `surface` band, no marker glyph |
| Footer action | `dim`; `text` when focused |

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
| `●` | Core connected |
| `›` | Path separator in the header |
| `·` | Secondary separator |

Agent state is always glyph plus word (`◐ working`), never color alone.

**Selection is the `surface` band plus a `text`-weight label — never a glyph.**
A selection marker and a state glyph in the same column would be read as the
same kind of thing, and today's UI already pays that price.

## Spacing

The cell is the unit. One blank row separates groups; never two. Indentation for
nesting is 2, 4 or 6 columns, never more. Rails truncate with `…`; they never
wrap.

## Component rules

- **Header**: one row. Left is where you are (`space · tab`), right is the core
  name, its connection glyph and the panel toggle. No center segment.
- **Rail**: on desktop the panel is a narrow always-visible rail on the **left**,
  separated from the terminal by a single rule. On mobile it is opened from the
  header; the collapsed face is the terminal with one header row.
- **Both faces**: a design of this panel always shows the collapsed and the open
  face. The collapsed face is the same in every direction.
- **Section**: one header row `▾ NAME  count`, then dense rows. A section with
  zero items renders its header and nothing else.
- **Row**: glyph, label, then the state word, right-aligned. A row never repeats
  a fact already in the header.
- **Footer actions**: one row at the bottom, `dim`, separated by two spaces. No
  labels such as "Collapse" where a glyph says it.
- **No status bar.** There is no bottom chrome row: connection state lives in
  the header, keyboard hints live in the terminal emulator's own key bar.

## Self-check

- Colors used by every artboard are drawn from the tables above. The artboards'
  `--accent`, `--surface`, `--panel`, `--bg`, `--text`, `--muted`, `--dim`,
  `--green`, `--yellow` and `--red` all match this document.
- No artboard uses a font family outside a system monospace stack.
- No artboard declares a size token, radius or shadow: the grid is the unit.
