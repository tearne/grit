# Worktree Naming

## Intent

Prevent concurrent sessions within the same repository from colliding on worktree paths. Currently, two sessions using the same ref (e.g. `grit HEAD~1 HEAD` and `grit HEAD~1 main`) would both claim `.grit/worktrees/HEAD-1/`, causing the second to fail or corrupt the first.

## Approach

Encode both refs in the worktree directory name: `.grit/worktrees/<ref-a>__<ref-b>/`. Each session gets a unique pair of worktrees, fully isolated from any other session in the same repo.

`sanitise_ref` is applied to each ref individually before joining with `__`. The separator `__` is chosen because `sanitise_ref` replaces `/`, `~`, `.`, `^`, `:` and other special characters with `-`, so `__` cannot appear naturally in a sanitised ref name and is unambiguous as a separator.

The working tree ref (`.`) sanitises to `.` today — `sanitise_ref` should treat it as a special case and map it to the literal string `working-tree` for clarity in directory names.

**Side effect**: `sanitise_ref` is also used for session IDs in `session.rs`. Changing `.` → `working-tree` renames existing session IDs that included `.`, causing those sessions to lose persisted review state on next run. Acceptable given the improved clarity and correctness.

**Version bump**: patch — 0.4.2.

**Review cadence**: end of change.

## Plan

## Log

Session state moved to `.grit/sessions/` subdirectory to give `.grit/` a clear type-based layout alongside `.grit/worktrees/`.

---

- [x] CHANGE `git.rs` — update `sanitise_ref` to map `.` → `working-tree`
- [x] CHANGE `main.rs` — compute session-scoped `worktrees_dir` as `<ref-a-sanitised>__<ref-b-sanitised>` before passing to `worktree::create`
- [x] CHANGE `Cargo.toml` — bump version to 0.4.2
- [x] REVIEW `SPEC.md` — update Worktree Management section
- [x] CHANGE `session.rs` — move session state from `.grit/<id>/` to `.grit/sessions/<id>/`
- [x] UPDATE `SPEC.md` — reflect sessions subdirectory

## Conclusion

Updated `sanitise_ref` in `git.rs` to map `.` → `working-tree`. Changed `main.rs` to compute a session-scoped `worktrees_dir` at `.grit/worktrees/<ref-a>__<ref-b>/`, eliminating collisions between concurrent sessions sharing a ref. Moved session state from `.grit/<id>/` to `.grit/sessions/<id>/`, giving `.grit/` a clean type-based layout. Existing `.grit/` directories are orphaned on upgrade — users should delete `.grit/` and run `git worktree prune` to migrate cleanly.
