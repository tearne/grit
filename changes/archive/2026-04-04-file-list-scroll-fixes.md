# File List Scroll Fixes

## Intent
Three gaps in the file list mouse interaction: clicking a file does not reliably select it and show its diff (scroll offset is not accounted for), the file list pane does not respond to mouse wheel events, and in tree mode the scrollbar thumb is sized and positioned based on the flat file count rather than the number of visual rows, making it inaccurate.

## Approach

**Click to select file**

`SelectVisualRow(row)` is emitted with `row` = the clicked row within the visible window (0-indexed from the top of the list), but the handler passes it directly to `select_index` (flat) or `tree.get` (tree) as if it were an absolute index. The fix is to add `list_state.offset()` at the call site in the event loop: `checklist.select_index(list_state.offset() + row)` for flat mode, and `tree.get(list_state.offset() + row)` for tree mode.

**Mouse scroll on file list**

`handle_mouse` in `tui.rs` currently only emits `ScrollPreview` for wheel events in the preview area (`mouse.row > divider_row`). Add two new arms guarded by `mouse.row > 0 && mouse.row < divider_row` that return a new `ChecklistControl::ScrollList(i32)` variant — `ScrollList(3)` for scroll-down, `ScrollList(-3)` for scroll-up. Handle `ScrollList(delta)` in the event loop by calling `checklist.select_next()` or `select_prev()` `delta.abs()` times, mirroring the `ScrollPreview` pattern.

**Tree-mode scrollbar row count**

`render_checklist` passes `total` (`session.files.len()`) to `scrollbar` regardless of view mode. Because `items` (the rendered list) is moved into `List::new(items)` before the `scrollbar` call, capture its length first: `let item_count = items.len();`. Pass `item_count` to `scrollbar` instead of `total`. This is correct for both modes: flat produces `session.files.len()` rows, tree produces `tree.len()` rows which includes directory entries.

Review cadence: single review at completion.

## Plan
- [x] UPDATE `tui.rs`: fix `SelectVisualRow` handler to add `list_state.offset()` to `row` in both flat and tree branches
- [x] UPDATE `tui.rs`: add `ScrollList(i32)` variant to `ChecklistControl`; add scroll-up/down arms to `handle_mouse` guarded by list area rows; handle `ScrollList(delta)` in the event loop
- [x] UPDATE `tui.rs`: capture `item_count = items.len()` before `List::new(items)` and pass it to `scrollbar` instead of `total`

## Log

Post-conclusion fix: mouse wheel in file list panel was not scrolling the list — `ScrollPreview` was being sent for all `mouse.row > 0` events, so wheel in the file panel drove the preview scroll. Reinstated `ScrollList(i32)` variant; split scroll arms on `divider_row` so wheel above the divider sends `ScrollList` and wheel below sends `ScrollPreview`. `ScrollList` handler calls `select_next`/`select_prev` by `delta.abs()` steps.

## Conclusion

Delivered as planned. Two post-review fixes: (1) file-panel mouse scroll was routing to preview — fixed by splitting scroll arms on `divider_row`; scroll wheel on file list was later removed entirely at user request, leaving click-to-select and scrollbar only. (2) `u`/`i` navigation in tree mode could not reach the last file — `select_next`/`select_prev` stepped by file index but tree displays files sorted by path; replaced with `SelectAdjacentFile(i32)` variant that walks the tree's visual row order.
