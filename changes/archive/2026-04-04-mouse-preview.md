# Mouse Preview

## Approach

Three new cases are added to `handle_mouse` in `tui.rs`:

**Scroll**: `MouseEventKind::ScrollDown` / `ScrollUp` when `mouse.row > divider_row` returns `ScrollPreview(3)` / `ScrollPreview(-3)`. Scroll events outside the preview area (i.e. in the list pane) are ignored — the list scrolls implicitly through file selection. The `ScrollPreview` control variant already exists and the run loop already handles it.

**Click to copy**: `MouseEventKind::Down(MouseButton::Left)` when `mouse.row > divider_row` — left half of the terminal (`mouse.column < width / 2`) returns `CopyPathOld`; right half returns `CopyPathNew`. Both control variants already exist and the run loop already handles them. `handle_mouse` gains a `width: u16` parameter to perform the split.

The `divider_row` boundary already distinguishes list from preview in the existing click handler, so the new cases slot in cleanly alongside it.

`SPEC.md` navigation section gains a mouse interactions subsection documenting scroll and click-to-copy behaviour.

Review cadence: single review at completion.

## Plan
- [x] UPDATE `tui.rs`: `handle_mouse` — add `width: u16` parameter; handle `ScrollDown`/`ScrollUp` in preview area returning `ScrollPreview(±3)`; handle left click in preview area returning `CopyPathOld` (left half) or `CopyPathNew` (right half)
- [x] UPDATE `tui.rs`: `handle_mouse` call site — pass `width`
- [x] UPDATE `SPEC.md` — document preview pane mouse interactions (scroll and click-to-copy)

## Intent
Mouse interactions on the diff preview pane: scrolling the mouse wheel scrolls the diff; clicking the left half copies the old-side file path (equivalent to `Y`); clicking the right half copies the new-side file path (equivalent to `y`).

## Conclusion

Delivered as planned. `width` propagation required adding it to `next_checklist_event` as well as `handle_mouse` — both call sites updated.
