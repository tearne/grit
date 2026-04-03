# Open Command

## Intent

The current `y`/`Y` action copies a hardcoded `:open path:line` string to the clipboard via OSC 52. This format is not useful in Helix because pasting in normal mode inserts into the buffer rather than opening the command prompt. Users need a configurable mechanism for "open this file at this line" that fits their editor and terminal setup. The feature should ship with at least two presets: one that copies just `path:line` to the clipboard via OSC 52, and one that sends a Helix open command directly to a Zellij pane.

Additionally, the line number in the fullscreen diff view is currently always 1 because `y`/`Y` is not bound there — it should reflect the current scroll position.

## Approach

### Config format

`OpenCommand` is an internally-tagged serde enum in `config.rs`, serialisable so the in-TUI save path can write it back:

```toml
[open_command]
preset = "osc52"

[open_command]
preset = "helix-zellij"
pane = "hx"
```

Omitting the `[open_command]` section defaults to `osc52`. The `Config` struct gains `#[derive(Serialize)]` to support writing back to `.grit.toml`.

### Open command execution

`clipboard.rs` is replaced by `open_command.rs` with a single `execute(command: &OpenCommand, path: &Path, line: u32)` function:

- `osc52` — OSC 52 sequence writing bare `path:line` (no `:open` prefix)
- `helix-zellij` — shells out to `zellij action write-chars` to send `:open path:line` to the named pane, then sends Enter via `zellij action write 13`. The exact flags (`--target-pane`, `--pane-name`, etc.) will be confirmed against `zellij action --help` during implementation.

### In-TUI config modal

Key `o` opens a modal overlay from either the checklist or diff view. The modal:

- Lists available presets; the current selection is highlighted
- Up/Down (or `i`/`u`) moves between presets
- When `helix-zellij` is selected, a text field for the pane name appears beneath the list; character input appends, Backspace deletes
- Enter saves to `.grit.toml` and closes; Esc cancels
- The modal is rendered as a centred bordered box over the current view

### `y`/`Y` in diff view

`run_diff_view` receives the open command and both worktree paths. `handle_diff_key` gains `CopyPathNew` and `CopyPathOld` variants (or equivalent inline handling). Line number = `scroll + 1`.

### `run` signature

`tui.run()` receives `config: &mut Config` instead of the bare `diff_tool: &str`, giving it access to both `diff_tool` and `open_command` and allowing the modal to mutate and save config.

### Version bump

0.4.3 → 0.5.0 (minor — new interactive feature).

### Review cadence

Per task.

## Plan

- [x] UPDATE SPEC — add `open_command` config field, in-TUI modal (`o` key), `y`/`Y` in diff view, key binding tables
- [x] ADD IMPL — `OpenCommand` enum in `config.rs`; add `Serialize` to `Config`; add deserialization tests
- [x] REWRITE — replace `clipboard.rs` with `open_command.rs`; `execute(command, path, line)` with `osc52` and `helix-zellij` implementations
- [x] UPDATE IMPL — `tui.run()` accepts `config: &mut Config` instead of bare `diff_tool`; update `main.rs` call site
- [x] ADD IMPL — in-TUI config modal: render, preset selection, pane-name text field, save-on-confirm
- [x] UPDATE IMPL — checklist `y`/`Y`: call `open_command::execute` instead of `clipboard::copy_open_command`
- [x] UPDATE IMPL — diff view `y`/`Y`: pass `open_command` and worktree paths into `run_diff_view`; line = `scroll + 1`
- [x] BUMP VERSION — 0.4.3 → 0.5.0

## Log

After initial implementation the user requested that feedback from zellij commands be visible in the TUI — stderr from failed commands (e.g. tab not found) was previously suppressed. Added a notification strip: `execute` now returns `Option<String>` (a short status message), the TUI stores it with a timestamp and renders it above the footer for ~4 seconds, replacing the normal footer while visible.

## Conclusion

Replaced `clipboard.rs` with `open_command.rs`. The helix-zellij preset was investigated and abandoned — `zellij action write-chars` targets the pane where the calling process is running, not the visually focused tab, so keystroke injection into another pane is not feasible with the current zellij action API.

The final implementation is OSC 52 only: `y`/`Y` copies ` path:line` (leading space, so the user can type `:open` in Helix and paste directly) to the clipboard. A brief yellow confirmation appears in the footer. `y`/`Y` in the diff view uses `scroll + 1` as the line number. The in-TUI config modal was removed as there is nothing left to configure. Version bumped to 0.5.1.
