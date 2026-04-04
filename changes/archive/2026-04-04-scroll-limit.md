# Scroll Limit

## Intent
When scrolling the diff preview, stop when the last diff line is a quarter of the way up the preview area. If the entire diff already fits in the preview area, no scrolling is permitted.

## Approach
The scroll limit in the `ScrollPreview` handler (`tui.rs`) currently sets `max = text.lines.len().saturating_sub(1)`, allowing the last diff line to reach the top of the preview. It needs to be replaced with a limit that leaves the bottom quarter of the preview empty.

The corrected limit: if `total_lines <= preview_height`, max is 0 (fits entirely, no scroll). Otherwise, `max = total_lines - preview_height + preview_height / 4`. At this scroll position the last line sits at visual row `preview_height - 1 - preview_height / 4` from the top, leaving `preview_height / 4` empty rows below it — a quarter of the area.

`preview_height` is not currently available in the event loop, but it can be derived from the two variables that are tracked there: `available_height - split_row`.

No changes needed to the resize or split-drag handlers; `preview_scroll` is already reset to 0 on file selection change, which is the most common reflow.

Review cadence: single task, review at completion.

## Plan
- [x] UPDATE IMPL `tui.rs` `ScrollPreview` handler: replace `max = text.lines.len().saturating_sub(1)` with the quarter-based limit using `available_height.saturating_sub(split_row) as usize` as `preview_height`

## Conclusion

Delivered as planned. No surprises.
