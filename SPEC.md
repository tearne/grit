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
```

## Behaviour

### Review Session

A session is the pairing of two refs. State is stored in `.grit/` at the repository root. An existing session for the same refs is resumed; otherwise a new one is created.

### Worktree Management

For each git ref, `grit` creates a worktree at `.grit/worktrees/<ref-sanitised>/` on session start and removes it on clean exit. Before removing a worktree, `grit` checks for uncommitted changes or unpushed commits; if either are present, the user is warned and prompted to confirm before removal. Unclean exits leave worktrees in place for reuse. The working tree ref (`.`) uses the repository's actual working directory — no worktree is created.

### File Checklist

Files differing between the two refs are listed with one of three states:

- **Unreviewed** — not yet reviewed
- **Reviewed, stable** — blob hashes on both sides match those recorded at review time
- **Reviewed, dirty** — reviewed, but at least one side is the working tree with uncommitted changes

Each row shows a status column, line-change counts, and the file path:

```
 [x] M  +123   -45  src/main.rs
 [ ] A    +8    -0  src/lib.rs
 [x] D    +0  -200  src/old.rs
 [ ] R    +3    -1  src/new.rs  (was: src/old_name.rs)
```

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

Per-file state records the blob hashes on both sides at review time, written to `.grit/<session-id>/state.toml`.

### Refresh

On refresh, `grit` recomputes the diff:

- Reviewed files with unchanged blob hashes remain **reviewed, stable**
- Reviewed files with changed blob hashes reset to **unreviewed**
- New files are added as **unreviewed**
- Files no longer in the diff are removed

### Clipboard Bridge

From the checklist or diff view, the user can copy a ready-to-paste editor command:

```
:open /absolute/path/to/worktree/src/file.rs:42
```

The path targets the chosen worktree side: `y` copies the `ref_b` (new) path; `Y` copies the `ref_a` (old) path. The line number reflects the diff view position, or 1 when invoked from the checklist. The command is written to the clipboard via OSC 52.

### Navigation

All TUI views support both keyboard and mouse navigation. Mouse clicks select items.

Checklist keyboard bindings:

| Key | Action |
|-----|--------|
| `j` / `↓` | Select next file |
| `k` / `↑` | Select previous file |
| `Enter` | Promote preview to fullscreen diff |
| `r` / `Space` | Toggle reviewed state |
| `y` | Copy `:open <path>:<line>` for `ref_b` (new) to clipboard |
| `Y` | Copy `:open <path>:<line>` for `ref_a` (old) to clipboard |
| `=` | Grow file list (move divider down) |
| `-` | Shrink file list (move divider up) |
| `Esc` / `q` / `Ctrl-C` | Quit |

`h` and `l` are not bound — the checklist is a single column so horizontal movement has no meaning.

Fullscreen diff keyboard bindings:

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
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
- Clipboard command targets the correct worktree for the active side
- Diff tool not on PATH → clear error before TUI opens
- Exiting with a worktree containing uncommitted changes → user is warned and prompted before removal
- Exiting with a worktree containing unpushed commits → user is warned and prompted before removal
