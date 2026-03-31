# Reuse Cached Diff for Fullscreen

## Intent

Pressing Enter on a file with a visible preview causes a noticeable delay because the fullscreen diff view re-captures the diff from scratch. Since the background worker has already computed and cached the diff for the preview pane, entering fullscreen should reuse that cached result directly — eliminating the delay. The fullscreen view is conceptually just the preview pane expanded to fill the screen with scrolling enabled.

## Approach

Add an `initial: Option<Text<'static>>` parameter to `run_diff_view`. When `Some`, skip the initial `capture_diff` call and use the provided text. When `None`, capture as before (handles the edge case where Enter is pressed before the cache is populated).

At the `OpenDiff` call site in `run`, pass `cached.get(&checklist.selected).cloned()`.

On resize inside `run_diff_view`, re-capture at the new full terminal width regardless — the cached text was rendered at preview width (`width - 1`) and needs to be redrawn at the correct width after a resize.

**Version bump**: patch — 0.4.1.

**Review cadence**: end of change.

## Plan

- [x] UPDATE `tui.rs` — add `initial: Option<Text<'static>>` to `run_diff_view`; pass `cached.get(&checklist.selected).cloned()` at call site
- [x] CHANGE `Cargo.toml` — bump version to 0.4.1

## Conclusion

Added `initial: Option<Text<'static>>` to `run_diff_view`. The `OpenDiff` handler passes `cached.get(&checklist.selected).cloned()`, so a pre-cached preview is used directly with no re-capture delay. If the cache is empty (Enter pressed before the worker finishes), it falls back to synchronous capture as before. Resize still re-captures at the new full width.
