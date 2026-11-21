# Upstream: what came from herdr-app

`herdr-app-tui` reuses herdr-app's client and model layers instead of
reimplementing them. This file is the contract for keeping that copy honest:
what was taken, what was changed, and how an update is applied.

## Pinned revision

| | |
|---|---|
| Repository | `https://github.com/herdrdev/herdr-app` |
| Revision | `317e673` |
| License | Apache-2.0, © Herdr, Inc. |

## What was copied

Every file below carries a header naming herdr-app and this revision.

| File | Lines | Local change |
|---|---:|---|
| `src/hc.rs` | 869 | One test removed (see D3) |
| `src/core_bin.rs` | 549 | None |
| `src/core_bin/cli_link.rs` | — | None |
| `src/model.rs` | 1129 | None |
| `src/layout.rs` | 919 | D1, D2 |
| `src/mirror.rs` | 359 | None |
| `src/persist.rs` | 119 | None |
| `src/spawn.rs` | 17 | None |
| `distribution/core-version` | 1 | None |
| `tests/fixtures/core-bin/` | 2 files | None |

Its `crate::mods::home()` call is satisfied by `src/mods.rs`, a three-line
shim (D4). Nothing else of herdr-app's mod host was taken.

## Local divergences

Every difference from upstream, so a sync knows exactly what it is re-applying.

| Id | File | Change | Why |
|---|---|---|---|
| D1 | `layout.rs` | `Content::Context.pane` is `Option<Box<PaneFrame>>` instead of `Option<PaneFrame>` | `clippy::large_enum_variant`, which this project denies. The wire shape is identical: serde sees through `Box` |
| D2 | `layout.rs` | `Node { size: sa, ..nb.clone() }` → `..nb` in `Layout::swap` | `clippy::redundant_clone`; the clone was dropped without further use |
| D3 | `hc.rs` | The `#[ignore]`d test `two_app_viewers_answer_queries_without_idle_size_theft` is removed | It needs `crate::vt` (the ghostty terminal state), which this crate does not have yet. Restore it with T5 |
| D4 | `src/mods.rs` | New file: `home()` only | `core_bin.rs` calls `crate::mods::home()`; herdr-app's `mods` is a 1,876-line mod host with 34 GPUI references, of which only this function is needed |

D1 and D2 exist only because this project denies clippy warnings without
allowing any. If upstream fixes them, drop the local change on the next sync.

## How to update

1. Read the upstream diff for the files above at the new revision.
2. Apply it to the copies here by hand. Never merge, never reformat the file.
3. Re-apply D1–D3 where they still apply; drop the ones upstream fixed.
4. Check that `src/mods.rs` still satisfies what `core_bin.rs` asks of `mods`.
5. Run `cargo test` and `cargo clippy --all-targets -- -D warnings`.
6. Update the revision in this file and in the header of every copied file.

The copies are pinned on purpose: a change from upstream arrives as a review.

## Endgame

The better shape is a `herdr-client` crate inside herdr-app's workspace holding
these modules, with both herdr-app and this TUI depending on it — one source of
truth and no copy at all. That is task T14 in `PROJECT.md`, and its first
requirement is that herdr-app builds again (its `vendor/ghostty` submodule is
uninitialised and Zig is needed for `ghostty-sys`).
