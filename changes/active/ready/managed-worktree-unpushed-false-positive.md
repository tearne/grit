# Managed Worktree Unpushed False Positive

## Intent
On every clean exit, users are incorrectly prompted to confirm removal of grit-managed worktrees because they appear to have unpushed commits, even when no commits have been made to them.

## Approach

`worktree::remove` calls `confirmed_safe_to_remove(&path)` before removing. The `confirmed_safe_to_remove` function runs `has_unpushed_commits`, which uses `git log HEAD --not --remotes=*`. On a detached HEAD with no remote (which is exactly what `git worktree add --detach` produces), this returns all commits in history — so the check always fires.

Managed worktrees are grit's own scratch space: they are never committed to by users and are always safe to remove. The fix is to short-circuit `confirmed_safe_to_remove` in `remove` for managed worktrees. `remove` already receives `repo_root`; `is_managed(&path, repo_root)` already exists. Add an early return in `remove` after extracting `path`: if `is_managed(&path, repo_root)`, proceed directly to `git worktree remove` without calling `confirmed_safe_to_remove`.

The existing test `has_unpushed_commits_returns_true_when_no_remote` remains valid — it documents why the function must not be called for managed worktrees.

Review cadence: single review at completion.

## Plan
- [ ] UPDATE `worktree.rs`: in `remove`, after extracting `path`, add an early return that skips `confirmed_safe_to_remove` when `is_managed(&path, repo_root)` is true

## Feedback

**Delivery status**: not delivered

Bug found during review: `confirmed_safe_to_remove` always prompts "has unpushed commits" for grit-managed worktrees, even when no commits have been made to them.

**Root cause**: grit creates worktrees with `git worktree add --detach`, producing a detached HEAD with no remote tracking branch. The `has_unpushed_commits` check runs `git log HEAD --not --remotes=*`, which returns every commit in history when no remote is configured — so it always fires.

**Impact**: users see a spurious prompt on every clean exit. The prompt now works correctly (raw mode fix in v0.5.11) but it fires incorrectly.

**Suggested fix**: `worktree::remove` already has access to `repo_root`. `worktree::is_managed(path, repo_root)` already exists and returns `true` for paths inside `.grit/worktrees/`. Skip the `has_unpushed_commits` check (or the entire `confirmed_safe_to_remove` check) for managed worktrees — they are grit's own scratch space and safe to remove unconditionally.
