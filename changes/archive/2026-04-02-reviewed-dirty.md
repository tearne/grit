# Reviewed, Dirty State

## Intent

When a reviewed file has the working tree as one side and that side has uncommitted changes, surface a distinct reviewed-dirty state in the checklist so the user knows the reviewed snapshot may no longer match what is on disk.

## Approach

`ReviewedDirty` is a runtime-computed overlay on top of the persisted `Reviewed` state — it is never stored in `state.toml`. On load, when either ref is `.`, `git diff-index --name-only HEAD` is called to obtain the set of files with uncommitted changes. Any file that would otherwise resolve to `ReviewedStable` but whose path appears in that set is promoted to `ReviewedDirty` instead. Between sessions the state persists as `Reviewed`; the dirty flag is re-evaluated fresh each time.

Toggle behaviour: `Unreviewed` → `ReviewedStable` or `ReviewedDirty` (depending on current dirty status of that file); `ReviewedStable` → `Unreviewed`; `ReviewedDirty` → `Unreviewed`.

Display: `[~]` for dirty, matching the convention that `[ ]` = unreviewed and `[x]` = reviewed stable.

Key implementation points:
- Add `ReviewedDirty` to `ReviewState` in `session.rs`
- Add `git::dirty_paths(repo_root) -> Result<HashSet<PathBuf>>` in `git.rs`
- `Session::load_or_create` calls `dirty_paths` when either ref is `.` and passes the result to `merge_state`
- `merge_state` gains a `dirty: &HashSet<PathBuf>` parameter; the `ReviewedStable` branch checks membership to decide between stable and dirty
- `to_persisted` maps `ReviewedDirty` to `PersistedState::Reviewed`
- TUI toggle and render updated for the new variant
- Review cadence: per-task

## Plan

- [x] ADD IMPL: `ReviewedDirty` variant to `ReviewState` in `session.rs`; update `to_persisted` to map it to `PersistedState::Reviewed`
- [x] ADD IMPL: `git::dirty_paths(repo_root) -> Result<HashSet<PathBuf>>` using `git diff-index --name-only HEAD`
- [x] UPDATE IMPL: `Session::load_or_create` to call `dirty_paths` when either ref is `.` and pass result to `merge_state`
- [x] UPDATE IMPL: `merge_state` to accept `dirty: &HashSet<PathBuf>` and produce `ReviewedDirty` when a would-be `ReviewedStable` file appears in the set
- [x] UPDATE IMPL: TUI render — `[~]` checkbox for `ReviewedDirty`
- [x] UPDATE IMPL: TUI toggle — `ReviewedDirty` → `Unreviewed`; marking reviewed on a dirty file → `ReviewedDirty`
- [x] ADD TEST: `merge_state` yields `ReviewedDirty` for a reviewed file in the dirty set
- [x] ADD TEST: `merge_state` yields `ReviewedStable` for a reviewed file not in the dirty set
- [x] UPDATE SPEC: add `[~]` checkbox character to the File Checklist display table

## Conclusion

Delivered as planned. A `dirty: bool` field was added to `FileEntry` (non-persisted) so that `toggle_reviewed` can produce the correct state without re-querying git. Existing tests were updated to pass an empty `HashSet` to `merge_state`; a shared `reviewed_persisted` helper was extracted to reduce duplication across the merge_state tests.
