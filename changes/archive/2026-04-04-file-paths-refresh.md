# File Paths Refresh

## Intent

After a refresh, the diff worker continues operating on the original file list. If files are added or removed, new files are never diffed and the index-based diff cache becomes misaligned with the session. The file path list must be recomputed from the session after each refresh.

## Approach

Extract a `build_file_paths` helper in `tui.rs` that takes `&[FileEntry]`, `worktree_a`, and `worktree_b` and returns `Vec<(PathBuf, PathBuf)>`. Call it once before the loop (as today) and again inside `ChecklistControl::Refresh` after `checklist.refresh()` returns, before `worker.reset()`. The other `worker.reset()` call sites (`CycleTheme`, `Resize`) do not change the file list so they do not need to recompute.

This change should be built before `deferred-refresh`, which restructures the refresh path.

Review cadence: at the end.

## Plan

- [x] ADD IMPL: `build_file_paths(files: &[FileEntry], worktree_a: &Path, worktree_b: &Path) -> Vec<(PathBuf, PathBuf)>` helper in `tui.rs`
- [x] UPDATE IMPL: checklist loop in `tui.rs` — replace the inline `file_paths` computation before the loop with a call to `build_file_paths`; call `build_file_paths` again in the `ChecklistControl::Refresh` arm after `checklist.refresh()` returns and before `worker.reset()`

## Conclusion

Delivered as planned. `file_paths` is now `mut` and recomputed from the updated session on each refresh, keeping the diff worker aligned with the current file list.
