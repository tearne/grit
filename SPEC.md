# Specification

## Overview

`grit` (Git Review In Terminal) is a terminal TUI for structured code review. It presents a checklist of files changed between two refs, tracks review progress persistently across sessions, and makes both sides of a diff fully accessible to your editor and language server via git worktrees.

## Usage

### Invocation

```
grit <ref-a> [ref-b]
```

`ref-a` is required. `ref-b` is optional and defaults to `.` (the working tree). Either may be a branch name, a tag, a commit SHA, or the working tree (`.`).

`grit --version` (or `-V`) prints the version and exits without launching the TUI.

### Configuration

Configuration is read from `.grit.toml` at the repository root. All fields are optional.

```toml
diff_tool = "difft --color always"  # default: difft --color always
theme = "autumn"                    # available: default, autumn; default: autumn
```

## Behaviour

### Review Session

A session is the pairing of two refs. State is stored in `.grit/` at the repository root. An existing session for the same refs is resumed; otherwise a new one is created.

### Worktree Management

For each git ref, `grit` creates a worktree at `.grit/worktrees/<ref-a-sanitised>__<ref-b-sanitised>/<ref-sanitised>/` on session start and removes it on clean exit. The session-scoped directory prevents concurrent sessions sharing a ref from colliding on worktree paths. The working tree ref (`.`) sanitises to `working-tree`. Before removing a worktree, `grit` checks for uncommitted changes or unpushed commits; if either are present, the user is warned and prompted to confirm before removal. Unclean exits leave worktrees in place for reuse. The working tree ref (`.`) uses the repository's actual working directory — no worktree is created within the session directory.

### File Checklist

Files differing between the two refs are listed with one of three states:

- **Unreviewed** — not yet reviewed
- **Reviewed, stable** — blob hashes on both sides match those recorded at review time
- **Reviewed, dirty** — reviewed, but at least one side is the working tree with uncommitted changes

Toggling an unreviewed file always produces **reviewed, stable** regardless of dirty status. The dirty overlay is applied by the diff/merge logic at load and refresh time only — it is never set directly by a toggle.

Each row shows a status column, line-change counts, and the file path:

```
 [x] M  +123   -45  src/main.rs
 [ ] A    +8    -0  src/lib.rs
 [~] D    +0  -200  src/old.rs
 [ ] R    +3    -1  src/new.rs  (was: src/old_name.rs)
```

Checkbox characters: `[ ]` unreviewed, `[x]` reviewed stable, `[~]` reviewed dirty.

Status characters: `A` added (green), `D` deleted (red), `M` modified (yellow), `R` renamed/moved (cyan). Line counts are right-aligned and colour-coded green (`+`) and red (`-`), with intensity scaling by magnitude: dim for zero, normal for 1–99, bold for 100–999, bold with coloured background for 1000+. Renamed files append `(was: <old_path>)` in dim style.

### Diff View

The TUI title bar shows `  {ref_a}  →  {ref_b}` on the left and `grit v{version}` flush to the right edge.

The checklist screen is split into a file list pane and a diff preview pane, separated by a draggable divider. The default split is 2/3 file list, 1/3 preview. Selecting a file immediately updates the preview; the preview always shows from the top with no user-controlled scrolling.

Pressing Enter promotes the preview to a fullscreen diff view where the user can scroll freely. The configured diff tool is invoked with the two worktree paths for that file; its output (including ANSI colour sequences) is captured and rendered by ratatui. The diff tool is passed `$COLUMNS` matching the current terminal width; on terminal resize the diff is re-captured at the new width and the scroll offset resets to zero.

Each pane has a 1-column scroll-indicator strip on the right edge showing `↑` when content is hidden above and `↓` when content is hidden below.

### Split Adjustment

The divider between the file list and preview panes can be repositioned:

- `=` — grow the file list (move divider down)
- `-` — shrink the file list (move divider up)
- Drag the divider row with the mouse

### Progress Persistence

Per-file state records the blob hashes on both sides at review time, written to `.grit/sessions/<session-id>/state.toml`.

### Refresh

On refresh, `grit` recomputes the diff:

- Reviewed files with unchanged blob hashes remain **reviewed, stable**
- Reviewed files with changed blob hashes reset to **unreviewed**
- New files are added as **unreviewed**
- Files no longer in the diff are removed

Refresh is triggered manually with `R` or automatically every 30 seconds. After each refresh the session is saved to disk.

### Clipboard

From the checklist or diff view, `y` copies `path:line` for the `ref_b` (new) side to the clipboard via OSC 52; `Y` copies for the `ref_a` (old) side. The line number reflects the current scroll position in the diff view, or 1 when invoked from the checklist. A brief confirmation appears in the footer. If the copied path resolves into a grit-managed worktree (`.grit/worktrees/…`), the confirmation is appended with a warning that edits there will be lost on exit.

### Navigation

All TUI views support both keyboard and mouse navigation. Mouse clicks select items.

Checklist keyboard bindings:

| Key | Action |
|-----|--------|
| `u` / `↓` | Select next file |
| `i` / `↑` | Select previous file |
| `j` | Scroll preview down |
| `k` | Scroll preview up |
| `Enter` | Promote preview to fullscreen diff |
| `r` / `Space` | Toggle reviewed state |
| `R` | Refresh (recompute diff) |
| `t` | Cycle theme |
| `y` | Copy `path:line` for `ref_b` (new) to clipboard |
| `Y` | Copy `path:line` for `ref_a` (old) to clipboard |
| `=` | Shrink file list (move divider up) |
| `-` | Grow file list (move divider down) |
| `Esc` / `q` / `Ctrl-C` | Quit |

`h` and `l` are not bound — the checklist is a single column so horizontal movement has no meaning.

Fullscreen diff keyboard bindings:

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `y` | Copy `path:line` for `ref_b` (new) to clipboard |
| `Y` | Copy `path:line` for `ref_a` (old) to clipboard |
| `Enter` / `q` / `Esc` / `Ctrl-C` | Return to checklist |

## Constraints

- Implemented in Rust
- Presented as a terminal TUI
- Both refs must resolve within the same repository
- `.` may only appear once
- The diff tool must be on `PATH`; a clear error is shown if it is not
- `.grit/` should be added to `.gitignore`

## Verification

- Two branch refs → worktrees created for both
- `.` as one ref → working directory used, no worktree created
- Review state survives exit and restores correctly on next invocation
- After a ref advances: unchanged files stay ticked, changed files reset to unreviewed
- A reviewed file with uncommitted working tree changes shows as reviewed, dirty
- `y`/`Y` writes `path:line` via OSC 52 and shows confirmation in footer
- `y`/`Y` in diff view uses the current scroll line, not 1
- Diff tool not on PATH → clear error before TUI opens
- Exiting with a worktree containing uncommitted changes → user is warned and prompted before removal
- Exiting with a worktree containing unpushed commits → user is warned and prompted before removal
