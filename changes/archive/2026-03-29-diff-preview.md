# Diff Preview

## Intent

Show a live diff preview in the lower third of the checklist screen. Navigating the file list immediately updates the preview. Pressing Enter promotes the preview to fullscreen, where the user can scroll freely and return to the checklist as before. The split point between the file list and preview is adjustable — by dragging the divider with the mouse or via keyboard shortcuts. Scroll indicators show when content is hidden above or below either pane.

## Approach

**Version bump**: minor — 0.3.0. This changes the primary screen layout.

**Layout**: the checklist screen gains a third pane between the file list and the footer:

```
┌─────────────────────────────┐  length 1
│ title bar                   │
├─────────────────────────────┤  split_row lines
│ file list                   │
├─────────────────────────────┤  divider (length 1)
│ diff preview                │  remaining lines
├─────────────────────────────┤  length 1
│ footer                      │
└─────────────────────────────┘
```

The divider row is a single line rendered distinctly (e.g. a horizontal rule style) to make it a visible drag target.

**Split state**: the run loop holds `split_row: u16` — the number of lines allocated to the file list pane (excluding title, divider, and footer). Default is 2/3 of available height, recalculated on resize to maintain the ratio. Minimum of 2 lines for each pane.

**Keyboard resize**: `=` grows the file list (moves split down), `-` shrinks it (moves split up), each by one line.

**Mouse resize**: on `MouseEventKind::Down(MouseButton::Left)` on the divider row, enter drag mode (`dragging: bool`). While dragging, `MouseEventKind::Drag` events update `split_row` to follow the cursor. `MouseEventKind::Up` ends the drag. The divider row is at `1 + split_row` (accounting for the title bar).

**Scroll indicators**: each pane is split horizontally into a 1-column indicator strip on the right and the main content area. The strip renders `↑` and/or `↓` at the top and bottom rows of the strip when content is hidden in that direction; otherwise blank.
- File list strip: `↑` if `ListState::offset() > 0`; `↓` if items extend below the visible area.
- Preview strip: `↓` if the captured diff has more lines than the pane height. Preview renders from offset 0 so `↑` is never shown.

**Preview capture**: difft is re-run whenever the selected file changes. The result is stored in a `preview: Option<Text<'static>>` held in the TUI run loop alongside the checklist. `capture_diff` is called with `COLUMNS` set to the current terminal width; on resize the preview is re-captured and `split_row` recalculated.

When no file is selected or the file list is empty, the preview pane is blank.

**Selection tracking**: the run loop tracks `previewed_index: Option<usize>`. After each event, if `checklist.selected` differs from `previewed_index`, re-capture and update.

**Enter behaviour**: unchanged — launches the fullscreen diff view for the selected file.

**Footer hints**: update to reflect `Enter` promotes to fullscreen; add `=`/`-` to hint line.

**Review cadence**: end of change.

## Plan

- [x] CHANGE `tui.rs` — add `preview`, `previewed_index`, `split_row`, `dragging` state to run loop; four-pane layout with divider; keyboard resize (`=`/`-`); mouse drag on divider; scroll indicators on file list and preview pane; re-capture on selection change and resize
- [x] CHANGE `Cargo.toml` — bump version to 0.3.0
- [x] REVIEW `SPEC.md` — update layout, Diff View, and Navigation sections

## Conclusion

Added a live diff preview pane below the file list, separated by a labelled divider (`─── Preview · Enter to expand ───`). Default split is 2/3 list, 1/3 preview. Selection changes trigger a difft re-capture automatically. The split is adjustable via `=`/`-` keys or mouse drag on the divider row. Each pane has a 1-column scroll-indicator strip showing `↑`/`↓` when content is hidden. Enter promotes to fullscreen diff. Version bumped to 0.3.0.
