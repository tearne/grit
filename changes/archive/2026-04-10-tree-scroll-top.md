# Tree Scroll Top

## Intent
In tree view, navigating up stops at the topmost file but cannot bring folder headers above it into view — those rows remain hidden above the viewport boundary with no way to reach them.

## Approach
The fix is in the `SelectAdjacentFile` handler in the tree event loop (`src/tui.rs`). When the selection is at `pos == 0` and `delta < 0`, `pos.saturating_sub(1)` returns 0 so `new_pos == pos` and the selection stays put — the viewport is never scrolled to reveal dir headers above.

When `new_pos == pos && delta < 0` (selection cannot move further up), decrement `list_state.offset()` by 1 instead of re-assigning the selection. This lets the user press `i` past the topmost file to scroll dir headers into view one line at a time.

## Log

Original approach was an auto-clamp after `render_stateful_widget` in `render_checklist`. User requested manual control instead: pressing `i` at the top scrolls the viewport up one line at a time.

## Plan
- [x] UPDATE IMPL in the `SelectAdjacentFile` tree branch, when the selection is already at the top (`new_pos == pos && delta < 0`), decrement `list_state.offset()` rather than re-assigning the selection

## Conclusion
Delivered. The original auto-clamp approach was replaced mid-build with manual viewport scrolling — pressing `i` at the topmost file now scrolls up one line at a time rather than auto-revealing headers.
