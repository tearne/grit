# Worktree Naming

## Intent

Prevent concurrent sessions within the same repository from colliding on worktree paths. Currently, two sessions using the same ref (e.g. `grit HEAD~1 HEAD` and `grit HEAD~1 main`) would both claim `.grit/worktrees/HEAD-1/`, causing the second to fail or corrupt the first.

## Approach

Encode both refs in the worktree directory name: `.grit/worktrees/<ref-a>__<ref-b>/`. Each session gets a unique pair of worktrees, fully isolated from any other session in the same repo.

`sanitise_ref` is applied to each ref individually before joining with `__`. The separator `__` is chosen because `sanitise_ref` replaces `/`, `~`, `.`, `^`, `:` and other special characters with `-`, so `__` cannot appear naturally in a sanitised ref name and is unambiguous as a separator.

The working tree ref (`.`) sanitises to `.` today — `sanitise_ref` should treat it as a special case and map it to the literal string `working-tree` for clarity in directory names.

**Review cadence**: end of change.

## Plan

- [ ] CHANGE `worktree.rs` — update `sanitise_ref` to map `.` → `working-tree`; update worktree path construction to use `<ref-a-sanitised>__<ref-b-sanitised>` as the directory name
- [ ] REVIEW `SPEC.md` — update Worktree Management section
