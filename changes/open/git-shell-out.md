# Git Shell-Out Strategy

## Intent

Evaluate whether to replace `git2` entirely with shelling out to the system `git` binary for all git operations. The motivation is simplicity: `git2`'s API has gaps (e.g. no detached-HEAD worktree creation), requires a native dependency, and duplicates logic the system `git` already handles correctly — including custom SSL certificates and local git config.

If adopted, this change would also be an opportunity to consolidate the `sanitise_ref` function, which is currently duplicated between `session.rs` and `worktree.rs`.
