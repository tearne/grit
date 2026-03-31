# Checklist Columns

## Intent

Add two columns to the file checklist: a status indicator (added, deleted, modified, moved) and line-change counts (`+N / -N`). Reviewers can gauge the size and nature of each change at a glance without opening the diff.

## Approach

**Version bump**: minor — 0.4.0.

**Data** (`git.rs`): introduce a `DiffEntry` struct and `FileStatus` enum (`Added`, `Deleted`, `Modified`, `Moved`) in `git.rs`. Change `diff_files` to return `Vec<DiffEntry>` instead of a tuple vec. `DiffEntry` carries `path`, `blob_a`, `blob_b`, `status`, `old_path: Option<PathBuf>`, `additions: u32`, `deletions: u32`.

Status and old path come from extending `parse_raw_line` to return the status character and both paths for renames. Line counts come from a second git command (`diff-index --numstat` / `diff-tree --numstat -r`) run alongside the existing `--raw` command, with results merged by destination path. The working-tree case (`.`) uses `diff-index`, consistent with existing code.

**Data model** (`session.rs`): add `status: FileStatus`, `old_path: Option<PathBuf>`, `additions: u32`, `deletions: u32` to `FileEntry`. These fields are always recomputed from git and are not persisted — no serde changes required. `merge_state` is updated to accept `Vec<DiffEntry>`.

**Display** (`tui.rs`): each checklist row becomes:

```
 [x] M  +123   -45  src/main.rs
 [ ] A    +8    -0  src/lib.rs
 [x] D    +0  -200  src/old.rs
 [ ] R    +3    -1  src/new.rs  (was: src/old_name.rs)
```

- Status column: single character — `A` (green), `D` (red), `M` (yellow), `R` (cyan)
- `+N` right-aligned in a fixed-width field, rendered green; `-N` rendered red
- For `Moved` files, append `  (was: <old_path>)` in dim style after the path

**Line-count intensity**: the colour intensity of `+N` and `-N` scales with magnitude across four levels:

- `0`: dim
- `1–99`: normal
- `100–999`: bold
- `1000+`: bold + coloured background (dark green for `+`, dark red for `-`)

**Review cadence**: end of change.

## Plan

- [x] CHANGE `git.rs` — introduce `FileStatus` enum and `DiffEntry` struct; extend `parse_raw_line` to return status and old path; run `--numstat` alongside `--raw` and merge by path; update `diff_files` return type
- [x] CHANGE `session.rs` — add `status`, `old_path`, `additions`, `deletions` to `FileEntry` (not persisted); update `merge_state` to accept `Vec<DiffEntry>`
- [x] CHANGE `tui.rs` — render status and line-count columns with colour and intensity scaling; append `(was: …)` for moved files
- [x] CHANGE `Cargo.toml` — bump version to 0.4.0
- [x] REVIEW `SPEC.md` — update File Checklist section

## Conclusion

Added `FileStatus` enum and `DiffEntry` struct to `git.rs`. `diff_files` now runs `--raw` and `--numstat` as separate commands and merges by position. `FileEntry` gains `status`, `old_path`, `additions`, and `deletions` — derived fresh from git each load, not persisted. Checklist rows now display status character (A/D/M/R with colour), right-aligned `+N`/`-N` counts with four-level intensity scaling (dim/normal/bold/bold+background), and `(was: …)` for renamed files. Column width is computed dynamically from the widest count in the list.
