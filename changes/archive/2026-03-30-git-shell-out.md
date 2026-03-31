# Git Shell-Out Strategy

## Intent

Evaluate whether to replace `git2` entirely with shelling out to the system `git` binary for all git operations. The motivation is simplicity: `git2`'s API has gaps (e.g. no detached-HEAD worktree creation), requires a native dependency, and duplicates logic the system `git` already handles correctly — including custom SSL certificates and local git config.

If adopted, this change would also be an opportunity to consolidate the `sanitise_ref` function, which is currently duplicated between `session.rs` and `worktree.rs`.

## Approach

`git2` is used in three places today:

- `main.rs` — `Repository::discover(".")` and `repo.workdir()` to locate the repo root
- `session.rs` — `compute_diff_files` and `resolve_tree` for diff computation and blob OID extraction
- `worktree.rs` — already shells out entirely; no `git2` usage

**Shell-out replacements:**

- Repo root: `git rev-parse --show-toplevel`
- Bare repo / not-a-repo errors: propagate stderr from the above command with the same user-facing messages as today
- Changed file list and blob OIDs: `git diff-tree --raw -r <ref_a> <ref_b>` for tree-to-tree, or `git diff-index --raw <ref_a>` for tree-to-worktree (`.` cases). These are plumbing commands used solely to build the checklist and track review stability — not to display diff content, which remains the diff tool's responsibility. The `--raw` format emits old and new blob OIDs per file; the new-side OID is all-zeros for working-tree files, which maps cleanly to the existing `None`-for-unstable-hash pattern used by `blobs_match`

**New module `git.rs`:** encapsulates all shell-out helpers and exposes two functions: `repo_root()` and `diff_files()`. `sanitise_ref` moves here, removing the duplication between `session.rs` and `worktree.rs`.

**Type change:** `Option<git2::Oid>` in `FileEntry` and the internal diff tuple becomes `Option<String>` (40-char hex). The `blobs_match` logic is structurally unchanged.

**`Cargo.toml`:** remove `git2`.

**Version bump:** patch — 0.3.2.

**Review cadence:** end of change.

## Plan

- [x] ADD `src/git.rs` — `repo_root()`, `diff_files()`, `sanitise_ref()`
- [x] UPDATE `src/session.rs` — replace `compute_diff_files` / `resolve_tree` with `git::diff_files()`; replace `Option<git2::Oid>` with `Option<String>`; remove local `sanitise_ref`
- [x] UPDATE `src/worktree.rs` — remove local `sanitise_ref`; use `git::sanitise_ref()`
- [x] UPDATE `src/main.rs` — replace `Repository::discover` / `repo.workdir()` with `git::repo_root()`; remove `git2` import
- [x] UPDATE `Cargo.toml` — remove `git2`; bump version to 0.3.2
- [x] REVIEW `SPEC.md` — no references to `git2` or specific dependencies; no update needed

## Conclusion

Implemented as planned. The `Option<String>` type change simplified `blobs_match` out of existence — blob comparison became a direct `==` on the field, so the helper was removed rather than left structurally unchanged as anticipated. All 9 tests pass.
