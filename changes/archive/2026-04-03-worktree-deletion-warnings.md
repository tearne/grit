# Worktree Deletion Warnings

## Intent

Before removing a worktree on exit, check for uncommitted changes and unpushed commits. If either are present, warn the user and require confirmation before proceeding with removal.

Additionally, when the user copies a file path using `y` or `Y` and that path resolves into a grit-managed worktree, warn them that any edits made there will be lost when grit exits, and suggest the safer alternative: check out the ref as a branch and review against the working tree with `grit <branch> .`.

## Approach

**Deletion warnings**

Both checks run only for `Worktree::Checkout`; `Worktree::WorkingTree` is skipped as before. The checks and the confirmation prompt are folded into `worktree::remove` so the call sites in `main.rs` are unchanged.

Two git commands, both run with `-C <worktree-path>`:
- Uncommitted changes: `git status --porcelain` — non-empty output means dirty.
- Unpushed commits: `git log HEAD --not --remotes='*' --oneline` — non-empty output means commits not reachable from any remote ref.

If either check finds a problem, the warnings are printed to stderr and the user is prompted (`[y/N]`). A `n` or empty response skips removal (leaves the worktree in place); `y` proceeds. The `--force` flag is retained so an affirmed removal always succeeds regardless of git's own safety checks.

Both helper functions (`has_uncommitted_changes`, `has_unpushed_commits`) live in `worktree.rs`. The stdin prompt is a small inline read in `remove` — not worth extracting.

**Copy path warning**

A managed worktree path starts with `repo_root.join(".grit/worktrees")`. A helper `worktree::is_managed(path, repo_root) -> bool` encapsulates this check. In `tui.rs`, the `CopyPathNew` and `CopyPathOld` handlers check whether the relevant worktree root is managed; if so, they append a warning to the notification string. The warning must fit the footer line, so it is kept short: `⚠ grit-managed worktree — edits will be lost on exit`. `tui.run()` does not need new parameters.

Review cadence: per-task.

## Plan

- [x] ADD IMPL: `has_uncommitted_changes(path: &Path) -> Result<bool>` in `worktree.rs` using `git -C <path> status --porcelain`
- [x] ADD IMPL: `has_unpushed_commits(path: &Path) -> Result<bool>` in `worktree.rs` using `git -C <path> log HEAD --not --remotes='*' --oneline`
- [x] UPDATE IMPL: `worktree::remove` — call both helpers for `Worktree::Checkout`, print warnings to stderr, prompt `[y/N]`, skip removal on non-`y` response
- [x] ADD TEST: `has_uncommitted_changes` returns `false` for a clean checkout
- [x] ADD TEST: `has_uncommitted_changes` returns `true` for a checkout with a dirty working tree
- [x] ADD TEST: `has_unpushed_commits` returns `false` when HEAD is reachable from a remote ref
- [x] ADD TEST: `has_unpushed_commits` returns `true` when HEAD is not reachable from any remote ref
- [x] ADD IMPL: `worktree::is_managed(worktree_root: &Path, repo_root: &Path) -> bool` — checks `starts_with(repo_root/.grit/worktrees)`
- [x] UPDATE IMPL: `CopyPathNew` and `CopyPathOld` handlers in `tui.rs` — append `⚠ grit-managed worktree — edits will be lost on exit` to the notification when `is_managed` is true
- [x] UPDATE SPEC: add behaviour bullet for the copy-path warning

## Conclusion

Delivered as planned. The prompt logic was extracted into `confirmed_safe_to_remove` returning `Result<bool>` so `remove` stays readable. The `has_unpushed_commits` "returns true" test uses a repo with no remote configured — sufficient to verify the logic without needing a push.
