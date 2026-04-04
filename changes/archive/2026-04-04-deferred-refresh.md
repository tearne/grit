# Deferred Refresh

## Intent

Auto-refresh currently applies changes immediately, which resets the preview scroll position and disrupts the user mid-review. Instead, the background timer should only detect whether the diff has changed. When a change is detected, a notification appears in the UI instructing the user to press `R`. The user triggers the refresh on their own terms, preserving scroll position until they choose to act.

## Approach

**Root cause**: the timer path in `tui.rs` sets `last_selected = None` after applying the refresh. On the next tick the guard at lines 88–94 fires (`last_selected != Some(checklist.selected)`) and resets `preview_scroll` to zero.

**Fix**: add `Session::has_pending_changes(repo_root)` in `session.rs`. It runs `diff_files` and `dirty_paths`, calls `merge_state` with the current `to_persisted()` snapshot, and compares the resulting file list against `self.files` by path set, blob hashes, and derived review states. Returns `true` if anything differs. This mirrors `refresh` exactly but is read-only.

In the TUI checklist loop, replace the timer-triggered `checklist.refresh()` call with `checklist.session.has_pending_changes(repo_root)?`. When that returns `true`, set a new `refresh_pending: bool` flag. `last_selected` is never reset by the timer — only by `R`. The footer renders "Changes detected — press R to refresh" with `footer_notification` style when `refresh_pending` is true, taking priority over timed notifications. `R` clears `refresh_pending` before applying the refresh as before.

The SPEC auto-refresh description is updated to reflect the new behaviour: the timer detects changes and shows a notification; `R` applies the refresh.

Review cadence: at the end.

## Plan

- [x] ADD IMPL: `Session::has_pending_changes(repo_root)` in `session.rs` — runs `diff_files` and `dirty_paths`, calls `merge_state` with `to_persisted()` snapshot, compares resulting paths, blob hashes, and states against `self.files`; returns `bool`
- [x] UPDATE IMPL: TUI checklist loop in `tui.rs` — replace timer-triggered `checklist.refresh()` with `checklist.session.has_pending_changes(repo_root)?`; set `refresh_pending: bool` when `true`; never reset `last_selected` from the timer path
- [x] UPDATE IMPL: `render_checklist` in `tui.rs` — when `refresh_pending` is `true`, display "Changes detected — press R to refresh" in the footer with `footer_notification` style, taking priority over timed notifications
- [x] UPDATE IMPL: `ChecklistControl::Refresh` handler in the main loop — clear `refresh_pending` before applying the refresh
- [x] ADD TEST: `has_pending_changes` returns `false` when diff is identical to current session state
- [x] ADD TEST: `has_pending_changes` returns `true` when a blob hash changes
- [x] ADD TEST: `has_pending_changes` returns `true` when a file is added to the diff
- [x] UPDATE SPEC: Refresh section — timer detects changes and shows a notification prompting `R`; `R` applies the refresh

## Conclusion

Timer no longer applies the refresh automatically. `Session::has_pending_changes` (backed by a private `differs` helper) does a read-only comparison; when it returns `true`, `refresh_pending` is set and the title bar shows "Changes detected — press r to refresh" on a red background (replacing the version text). Tests exercise `differs` directly via a `cfg(test)` export — the git I/O path is not mocked but the comparison logic is fully covered. 31 tests passing.

Review adjustments: refresh key moved from `R` to `r`; space on `~` now advances to `x` rather than clearing to `[ ]`; notification moved to title bar (red background, right-justified) and removed from footer.
