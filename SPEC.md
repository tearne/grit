# Specification

## Overview

`grit` (Git Review In Terminal) is a terminal TUI for structured code review. It presents a checklist of files changed between two refs, tracks review progress persistently across sessions, and makes both sides of a diff fully accessible to your editor and language server via git worktrees.

## Usage

### Invocation

```
grit <ref-a> <ref-b>
```

Both refs are required. Either may be a branch name, a tag, a commit SHA, or the working tree (`.`).

### Configuration

Configuration is read from `.grit.toml` at the repository root. All fields are optional.

```toml
diff_tool = "difft --color always"  # default: difft
pager     = "less -R"               # default: less -R
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

### Diff View

Selecting a file renders its diff via the configured diff tool, invoked with the two worktree paths for that file.

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

The path targets the worktree for the currently active side. The line number reflects the diff view position, or 1 when invoked from the checklist. The command is written to the clipboard via OSC 52.

### Navigation

All TUI views support both keyboard and mouse navigation. Mouse clicks select items.

Keyboard bindings:

| Key | Action |
|-----|--------|
| `j` / `↓` | Select next file |
| `k` / `↑` | Select previous file |
| `Enter` | Open diff for selected file |
| `r` / `Space` | Toggle reviewed state |
| `y` | Copy `:open <path>:<line>` command to clipboard |
| `q` / `Esc` / `Ctrl-C` | Quit |

`h` and `l` are not bound — the checklist is a single column so horizontal movement has no meaning.

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
