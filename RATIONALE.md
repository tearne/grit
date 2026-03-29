# Rationale

## The Problem

Code review against git history has two hard requirements that no existing terminal tool satisfies together.

**Structured progress tracking.** You need to work through changed files one at a time, tick them off, take breaks, and resume without losing your place. Branches also evolve mid-review — you need to know which already-reviewed files have changed and need another look.

**Full editor access on both sides of the diff.** Understanding a change often means following a call chain, checking what a type used to be, or running tests against the old version. This requires LSP navigation, compilation, and editing on both sides simultaneously. Tools that only expose the checked-out branch leave half the picture inaccessible. GitLens comes closest but suffers from this limitation.

## The Solution

`grit` is a terminal TUI for structured code review built around three ideas:

- **Worktrees for both sides.** Both refs are checked out on disk simultaneously, giving the editor and language server full access to each.
- **Persistent, accurate progress.** Review state is saved by blob hash so it survives sessions and remains correct as refs move forward.
- **Composition over integration.** The diff tool, editor, and terminal layout are all external. `grit` hands off rather than absorbs.

## Design Philosophy

`grit` follows the Unix philosophy: do one thing well and compose with other tools rather than absorbing them. It owns the review session — the checklist, the state, the worktrees — and nothing more. The clipboard bridge is the canonical example: instead of integrating with an editor, `grit` hands off a ready-to-paste command and lets the editor do its job.

Future proposals should be held against this principle. The question is not "would this be useful?" but "does this belong in `grit`, or in a tool it composes with?"

## Git Integration Strategy

`grit` uses `git2` (with default transports disabled) for local repository operations — worktree management, blob hash resolution, diff computation — and shells out to the system `git` binary for any network operations (e.g. `git fetch`).

The reason for shelling out rather than using `git2`'s built-in SSH/HTTPS transports is that the system `git` binary inherits the host's full git configuration, including any custom SSL certificate authorities. Reimplementing that trust chain inside `git2` is fragile and would silently break in environments with custom PKI.
