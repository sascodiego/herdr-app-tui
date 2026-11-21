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
| D5 | `src/vt.rs`, `src/links.rs`, `src/win32_keys.rs`, `crates/ghostty-sys/` | FFI `unsafe` to libghostty's C ABI | Documented exception to `AGENTS.md`'s no-unsafe rule, authorised by the maintainer on 2026-11-18. Required for Kitty graphics, terminal queries and key encoding — parity with herdr-app. Local lint fixes: parenthesised precedence in `win32_keys`, `SeenRows` alias, typed `transmute::<F, *const c_void>`, redundant clones |
| D6 | `crates/ghostty-sys/build.rs` | Ghostty source is `GHOSTTY_SRC`, default `../../../herdr-app/vendor/ghostty` | Upstream hardcodes `../../vendor/ghostty` inside herdr-app's own workspace; this crate lives in another repo |
| D8 | `src/lib.rs::test_tmp` | New test-only helper: `HERDR_TEST_TMPDIR` or `/var/tmp` | Copied `links.rs` tests call `crate::test_tmp()`, which lived in herdr-app's `main.rs` |
| D9 | `src/vt.rs::select`, `encode_mouse` | Floats grouped into `SelectPoint` and a `(x, y)` tuple | `clippy::too_many_arguments`; no `#[allow]` used. Test call sites updated |
| D10 | `src/spawn.rs::command` | `mut` only on the Windows branch | Upstream's `#[allow(unused_mut)]` was removed: this project forbids allow attributes |
| D7 | `src/sprites.rs`, `src/settings.rs` | New shims: only `is_sprite`, `theme`, `rgb` and `get().minimum_contrast` | The real modules are 516 and 1,090 lines with GPUI; `vt.rs` only reads these four items |
| D11 | `src/tui/app.rs` | `App` no longer derives `Debug`; the panel's rows come from the shared document via `Document::replace` | Upstream's `Model` is not `Debug`, and herdr-app reads spaces/tabs from `doc.get` rather than from `terminal.list`. Adding `Debug` to the copied model or pulling in `mirror_sync.rs`/`layout_sync.rs` would widen the copy for no benefit |

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
truth and no copy at all. That is task T14 in `PROJECT.md`.

## Toolchain for D5

`ghostty-sys` builds libghostty-vt from the pinned submodule with Zig, so it
is the only part of this project that needs an external toolchain:

| Requirement | Value |
|---|---|
| Zig | 0.16.0 (the ghostty pin declares `minimum_zig_version = "0.16.0"`) |
| Installed at | `~/.local/zig/zig-aarch64-linux-0.16.0/`, symlinked from `~/.local/bin/zig` |
| Tarball SHA-256 | `ea4b09bfb22ec6f6c6ceac57ab63efb6b46e17ab08d21f69f3a48b38e1534f17` |
| Ghostty source | `herdr-app/vendor/ghostty` at `291fd050b9e9c293c8adb6a140742649b7e42d3a` |
| Env | `ZIG` or `PATH` must name that binary; first build compiles wuffs, uucode and SIMD and takes minutes |

No sudo was needed: both live under `$HOME` or the repository submodule.
