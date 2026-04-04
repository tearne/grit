# Scrollbar

## Intent
Replace the current scroll arrows on the file list and diff preview panes with proportional scrollbars, so the scroll position and remaining content are visible at a glance. When all content fits in the pane, no scrollbar is shown.

## Approach

The existing `scroll_indicator(above, below, height)` function is replaced with `scrollbar(total, offset, visible, height)`. When `total <= visible` it returns an empty `Text` (no bar rendered). Otherwise it renders a proportional thumb:

- **Thumb height**: `max(1, visible * height / total)` — scales with how much of the content is visible
- **Thumb position**: `offset * (height - thumb_h) / (total - visible)` — maps scroll offset to track position
- **Characters**: `█` for thumb cells, space for track cells

Both call sites are updated:
- List pane: `total` = file count, `offset` = `list_state.offset()`, `visible` = `list_height`
- Preview pane: `total` = `text.lines.len()`, `offset` = `preview_scroll`, `visible` = `preview_height`

The 1-column indicator strip that already exists on both panes is reused unchanged — only the function producing its content changes.

Review cadence: single review at completion.

## Plan
- [x] UPDATE `tui.rs`: replace `scroll_indicator` with `scrollbar(total, offset, visible, height)` implementing proportional thumb; update both call sites (list pane and preview pane)

## Feedback

**Delivery status**: delivered

Two gaps surfaced during review:

1. **Mouse scroll on the file list** — the file list pane has no mouse wheel support. The preview pane gained scroll via `mouse-preview` but the file list was not covered.
2. **Scrollbar in tree view** — the list pane scroll indicator column currently uses `total` = `session.files.len()` (flat file count) rather than the number of visual rows rendered in tree mode. In tree mode the row count is higher (includes directory rows), so the scrollbar thumb is incorrectly sized and positioned.

Suggested next steps for the planner: add a change covering mouse scroll on the file list, and fix the tree-mode scrollbar to use the visual row count.
