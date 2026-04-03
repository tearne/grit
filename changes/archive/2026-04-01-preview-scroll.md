# Preview Scroll

## Intent

The preview pane currently always shows a diff from the top with no scrolling. Adding scroll support to the preview pane makes pressing Enter a natural promotion to fullscreen — same content, same position, just expanded. The two views should feel like one continuous experience.

## Approach

Add `preview_scroll: usize` to the `run` loop state, reset to `0` whenever the selection changes. `u` scrolls the preview up, `i` scrolls it down.

Pass `preview_scroll` to `render_checklist` and use it in `Paragraph::scroll((preview_scroll as u16, 0))`. Update the scroll indicator to reflect the current offset (`above = preview_scroll > 0`; `below = preview_scroll + pane_height < lines`).

Pass `preview_scroll` as the initial scroll offset to `run_diff_view`, replacing the hardcoded `let mut scroll = 0`. This carries the preview position seamlessly into fullscreen.

Scrolling is a no-op when the diff is still loading (no cached text for the selected file).

**Version bump**: patch — 0.4.3.

**Review cadence**: end of change.

## Plan

- [x] UPDATE `tui.rs` — add `u`/`i` to `ChecklistControl`; handle in `run` to adjust `preview_scroll`; reset on selection change
- [x] UPDATE `tui.rs` — pass `preview_scroll` to `render_checklist`; apply scroll and fix scroll indicator in preview pane
- [x] UPDATE `tui.rs` — pass `preview_scroll` as initial scroll to `run_diff_view`; add `initial_scroll: usize` parameter
- [x] UPDATE `SPEC.md` — document `u`/`i` bindings in checklist key table
- [x] CHANGE `Cargo.toml` — bump version to 0.4.3

## Log

- Swapped `j`/`k` and `u`/`i`: `j`/`k` now scroll preview and fullscreen diff; `u`/`i` navigate files. Keeps scrolling keys consistent across both views.
- `run_diff_view` changed to return `Result<usize>` (final scroll position) so preview scroll is preserved on return from fullscreen.
- Swapped `-`/`=` split adjustment: `-` grows the file list, `=` shrinks it.

## Conclusion

Added `preview_scroll: usize` state to the checklist loop, reset on selection change. `j`/`k` scroll the preview; scroll position carries into fullscreen on Enter and is restored on return. `u`/`i` and arrow keys navigate files. `-`/`=` split adjustment direction swapped to match user expectation.
