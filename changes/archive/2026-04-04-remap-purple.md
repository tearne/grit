# Remap Purple

## Approach

`Theme` gains a `diff_unmatched: Color` field — the colour used for difftastic's syntax-unpositioned characters. Sensible defaults:
- `default` theme: `Color::Magenta` (preserves existing behaviour)
- `autumn` theme: a muted lavender that sits comfortably alongside the warm palette, e.g. `Color::Rgb(0xBD, 0x82, 0xD7)`

`ansi.rs`: `parse` and `apply_sgr`/`remap` gain a `diff_unmatched: Color` parameter. The `remap` function adds arms for `Color::Magenta | Color::LightMagenta → diff_unmatched`. Both standard (35) and bright (95) magenta are covered.

`tui.rs`: the `parse` call site passes `theme.diff_unmatched`.

A test is added to `ansi.rs` covering magenta foreground remapping, parallel to the existing green and red tests.

Review cadence: per-task.

## Plan
- [x] UPDATE `theme.rs`: add `diff_unmatched: Color` to `Theme`; set per theme (`default`: `Color::Magenta`, `autumn`: muted lavender)
- [x] UPDATE `ansi.rs`: add `diff_unmatched` parameter to `parse`; thread through `apply_sgr` to `remap`; add `Magenta | LightMagenta` arms to `remap`
- [x] UPDATE `tui.rs`: pass `theme.diff_unmatched` at the `parse` call site
- [x] ADD `ansi.rs` test: magenta foreground remaps to `diff_unmatched`

## Conclusion

Delivered as planned. Threading `diff_unmatched` required updating `WorkItem`, `work_items`, and `capture_diff` in addition to the stated call site — the colour flows through the diff worker pipeline. Two tests added: standard magenta (ESC[35m) and bright magenta (ESC[95m). All 34 tests pass.

**Fix (post-build):** `worktree::remove` was called while `tui` was still in scope, leaving the terminal in raw mode. `read_line` in the `[y/N]` prompt then blocked indefinitely, hanging the process. Added `drop(tui)` before the remove calls in `main.rs` to restore cooked mode first. v0.5.10 → v0.5.11.

## Intent
Difftastic uses purple/magenta to mark characters it cannot place in the parsed syntax tree. This colour currently passes through grit's ANSI remapping unmodified, clashing with the active theme. It should be remapped alongside the existing added/deleted colours so the diff output feels consistent.
