# Initial Implementation

## Intent

Deliver a working thin vertical slice of `grit`: enough to complete a real review session end-to-end. The goal is the core loop — start a session, navigate the file list, view diffs, mark files reviewed, exit and resume.

In scope for this slice:

- `grit <ref-a> <ref-b>` invocation
- Worktree creation for git refs; working directory passthrough for `.`
- File checklist TUI with unreviewed / reviewed-stable states, supporting vi-style keyboard navigation and mouse
- Diff view via configured diff tool (default: difft)
- Mark file reviewed / unreviewed
- Persist and restore session state across exits and resumes
- Clipboard bridge (`:open <path>:<line>`)
- `.grit.toml` configuration for diff tool

Intentionally deferred:

- Refresh (re-diff after a ref advances)
- Reviewed, dirty state (working tree uncommitted changes)
- Worktree deletion warnings (uncommitted changes / unpushed commits)

## Approach

**Version**: 0.1.0.

**Crates**: `ratatui` for the TUI (keyboard and mouse event handling included), `git2` (default transports disabled) for local git operations (worktree creation, blob hash resolution, ref diffing), `clap` for CLI argument parsing, `serde` + `toml` for config and state, `color_eyre` for error reporting. Network git operations (e.g. fetch) shell out to the system `git` binary to inherit the host's SSL certificate configuration.

**Module structure**:
- `session` — resolve refs, compute file diff, load and save review state
- `worktree` — create and remove git worktrees via `git2`
- `checklist` — file list and review state transitions
- `tui` — ratatui render loop, keyboard and mouse event dispatch
- `config` — `.grit.toml` parsing
- `clipboard` — write OSC 52 escape sequence directly to the terminal

**Session identity**: keyed by the two ref names, sanitised and joined (e.g. `main__feature-x`). This means the same session is resumed when a branch advances; blob hashes determine whether individual files remain reviewed-stable.

**Diff tool invocation**: the TUI is suspended, the diff tool is spawned via `std::process::Command` with the two worktree-relative file paths, and the TUI is restored on return.

**Worktree removal**: removed on clean exit with no warnings (deletion warnings are deferred). Unclean exits leave worktrees in place for reuse.

**Review cadence**: per-task.

## Plan

- [x] ADD Cargo project scaffold — `Cargo.toml` with all dependencies (`ratatui`, `git2`, `clap`, `serde`, `toml`, `color_eyre`), `src/main.rs` entry point, `.gitignore` with `.grit/`
- [x] ADD `config` module — parse `.grit.toml`; surface a clear error if diff tool is not on `PATH`
- [x] ADD `worktree` module — create and remove git worktrees via `git2`; working-tree passthrough for `.`
- [x] ADD `session` module — session identity (sanitised ref pair), file diff computation via `git2`, load and save `state.toml`, reviewed-stable determination by blob hash comparison
- [x] ADD `checklist` module — file list data model, review state transitions (unreviewed ↔ reviewed-stable)
- [x] ADD `tui` module — ratatui render loop, checklist view with vi-style keyboard and mouse navigation
- [x] ADD diff tool invocation — suspend TUI, spawn diff tool with worktree-relative paths, restore TUI on return
- [x] ADD `clipboard` module — OSC 52 bridge, `:open <path>:<line>` command
- [x] REVIEW end-to-end: two branch refs, `.` passthrough, state persistence, diff tool missing error
- [x] REVIEW `SPEC.md` — verify it accurately reflects the delivered behaviour; update any gaps or inaccuracies surfaced during implementation
