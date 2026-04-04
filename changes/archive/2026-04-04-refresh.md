# Refresh

## Intent

When a ref advances mid-review, recompute the diff and update file states: unchanged files remain reviewed, changed files reset to unreviewed, new files are added, and removed files are dropped.

## Approach

Refresh re-runs `git::diff_files` and `merge_state` against the live refs, using the current in-memory `session.files` as the prior state (not disk — the user may have toggled files since the last save). `Session::to_persisted()` already serialises the in-memory state in the format `merge_state` expects, so the flow is: snapshot current state → diff → re-merge → replace `session.files`. If either ref is `.`, dirty paths are re-evaluated as part of the merge (depends on the `reviewed-dirty` change being built first, which introduces `dirty_paths` and the updated `merge_state` signature).

Triggered by `R` in the checklist view, or automatically every 30 seconds. The TUI event loop already uses `event::poll` with a short timeout; a `last_refresh: Instant` timestamp is added to the loop state and checked on each tick. After refresh, `selected` is clamped to the new file list length and the session is saved to disk.

The SPEC currently has no keybinding entry for refresh and does not describe auto-refresh — both are added. The Refresh section's behaviour rules are already correct.

Key implementation points:
- `Session::refresh(repo_root)` snapshots `to_persisted()`, re-runs diff and dirty, calls `merge_state`, replaces `self.files`
- `Checklist::refresh(repo_root)` delegates to `session.refresh` then clamps `selected`
- TUI checklist loop tracks `last_refresh: Instant`; refreshes on `R` keypress or when 30 seconds have elapsed since `last_refresh`
- After refresh: save session, reset `last_refresh`
- Review cadence: per-task

## Plan

- [x] ADD IMPL: `Session::refresh(repo_root)` in `session.rs`
- [x] ADD IMPL: `Checklist::refresh(repo_root)` in `checklist.rs` — delegates to session, clamps `selected`
- [x] UPDATE IMPL: TUI checklist event loop — track `last_refresh: Instant`, trigger refresh on `R` or after 30 seconds, save session after each refresh
- [x] ADD TEST: refresh keeps `ReviewedStable` for unchanged blobs
- [x] ADD TEST: refresh resets to `Unreviewed` for changed blobs
- [x] ADD TEST: refresh adds new files as `Unreviewed`
- [x] ADD TEST: refresh removes files no longer in the diff
- [x] UPDATE SPEC: add `R` — Refresh to the checklist keyboard bindings table; describe 30-second auto-refresh in the Refresh section

## Log

During testing, toggling an unreviewed dirty file immediately produced `[~]` instead of `[x]`. The `reviewed-dirty` approach specified `Unreviewed → ReviewedDirty` when the file is dirty at toggle time, but this is confusing UX: the user just reviewed the file and gets told it's in a suspect state before anything has changed. The `[~]` state is more useful as "reviewed earlier, but the working tree has since changed." Fixed by always toggling `Unreviewed → ReviewedStable` and letting the dirty overlay apply only at load/refresh time. The `dirty` field on `FileEntry`, added to support the toggle check, was also removed as it became unused.

## Conclusion

Delivered as planned. Refresh tests exercise the full snapshot→merge flow by calling `to_persisted()` on a constructed `Session` and passing the result to `merge_state` alongside a new diff, mirroring exactly what `Session::refresh` does at runtime. A `snapshot` helper was added to the test module to reduce construction boilerplate. The auto-refresh check runs before `next_checklist_event` so a stale timer is caught even when the user is idle.

One deviation from the `reviewed-dirty` approach: `toggle_reviewed` now always produces `ReviewedStable` from `Unreviewed`, not `ReviewedDirty`. See Log and Feedback.

## Feedback

**Delivery status**: delivered

The `reviewed-dirty` approach specified that toggling an unreviewed dirty file should produce `ReviewedDirty`. In practice this is confusing — the user marks something reviewed and immediately sees `[~]`, before anything has changed under them. The `[~]` state is only meaningful as "you reviewed this, and the working tree has since moved on." The fix (toggle always → `ReviewedStable`) was applied during this build.

The approach for `reviewed-dirty` should be updated to reflect the correct toggle behaviour: `Unreviewed → ReviewedStable` always. The dirty overlay belongs in `merge_state` only.
