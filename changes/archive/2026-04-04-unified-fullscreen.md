# Unified Fullscreen

## Intent

The current fullscreen diff view is a separate mode with its own keyboard bindings and scroll state. Pressing Enter should instead maximise the preview pane within the existing checklist view, and pressing Enter again should restore the previous split position. This gives a single consistent interaction model — one loop, one scroll position, one set of bindings.

## Approach

**Mechanism**: add `saved_split: Option<u16>` to the checklist loop state. `None` = normal mode; `Some(n)` = fullscreen mode, `n` is the split to restore. Enter toggles: if `saved_split.is_none()`, save current `split_row` into `saved_split` and set `split_row = 0`; if `saved_split.is_some()`, restore `split_row` from `saved_split` and clear it. `ChecklistControl::OpenDiff` is renamed `ToggleFullscreen`.

**Layout in fullscreen**: `split_row = 0` gives the file list a zero-height constraint, making it invisible. The divider remains at its usual position (row 1, below the title bar). Its text changes to show the selected file path and a restore hint: `─── <path> · Enter to restore `. This provides file context without a separate title bar. The preview and footer are unchanged.

**Split adjustment**: `AdjustSplit` (`=`/`-`) and mouse drag are ignored while `saved_split.is_some()`. Drag start on the divider row is also ignored in fullscreen.

**Copy line number**: `y`/`Y` currently pass line 1 when invoked from the checklist and `scroll + 1` from the diff view. With a single pathway, both use `preview_scroll + 1` always. This is a minor behaviour change: `y` from the non-fullscreen checklist now copies `preview_scroll + 1` rather than always 1. The SPEC `y`/`Y` description is updated accordingly.

**Removed**: `run_diff_view`, `render_diff`, `next_diff_event`, `handle_diff_key`, `DiffControl`. The fullscreen diff keyboard bindings table in the SPEC is removed; the checklist bindings table is updated.

**Ordering**: this change should be built before `deferred-refresh`. The Enter analysis note in `deferred-refresh`'s Approach becomes moot and should be removed before that change is handed off.

Review cadence: at the end.

## Plan

- [x] UPDATE SPEC: Diff View section — Enter maximises the preview pane and restores on second press; remove the fullscreen diff keyboard bindings table; update the checklist bindings table (Enter row); update `y`/`Y` description to "always uses the current preview scroll position"
- [x] REMOVE IMPL: `run_diff_view`, `render_diff`, `next_diff_event`, `handle_diff_key`, `DiffControl` from `tui.rs`
- [x] UPDATE IMPL: rename `ChecklistControl::OpenDiff` → `ToggleFullscreen`; add `saved_split: Option<u16>` to checklist loop state; on `ToggleFullscreen` — if `saved_split.is_none()` save `split_row` and set it to `0`, else restore from `saved_split` and clear it
- [x] UPDATE IMPL: `render_checklist` — accept a `fullscreen: bool` parameter; when `true`, replace the left divider label with the selected file path and change the hint to `· Enter to restore`
- [x] UPDATE IMPL: main loop `AdjustSplit` arm — skip when `saved_split.is_some()`
- [x] UPDATE IMPL: `handle_mouse` — accept `fullscreen: bool`; skip setting `dragging = true` when fullscreen and the cursor is on the divider row
- [x] UPDATE IMPL: `CopyPathNew` / `CopyPathOld` handlers in the main loop — pass `(preview_scroll + 1) as u32` instead of `1`

## Conclusion

Fullscreen diff mode removed. Enter now toggles `saved_split` to collapse/restore the file list within the single checklist view. `run_diff_view`, `render_diff`, `next_diff_event`, `handle_diff_key`, and `DiffControl` removed. Divider label shows file path and restore hint when fullscreen. `AdjustSplit` and mouse drag on the divider are ignored in fullscreen. `y`/`Y` now always use `preview_scroll + 1`. 28 tests passing.
