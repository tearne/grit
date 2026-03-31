# Internal Diff Viewer

## Intent

Replace the external pager with an internal diff view rendered by ratatui. The user opens a diff with Enter, navigates it with `j`/`k`, and returns to the checklist with `q`, `Enter`, or `Esc` — symmetric with how they opened it. difft is still used to produce the diff output.

## Approach

**Version**: 0.2.0.

**New dependency**: `ansi-to-tui` was the original plan, but it was removed in favour of a custom `src/ansi.rs` parser — `ansi-to-tui` v8 returns `ratatui_core::text::Text` which is a distinct, incompatible type from `ratatui 0.29`'s `Text`.

**Removed**: the `pager` config field and its PATH validation. Users no longer need `less` installed.

**Diff view state** (new, inside `tui.rs`):
- Captured difft output parsed into ratatui `Text`
- Vertical scroll offset (`usize`)
- Render: full-screen `Paragraph` with scroll, title bar showing the file path, footer showing key hints

**Key bindings in diff view**:
- `j` / `↓` — scroll down
- `k` / `↑` — scroll up
- `q` / `Enter` / `Esc` — return to checklist

**Terminal resize**: on `Event::Resize`, re-run difft and re-capture at the new width. Scroll offset resets to 0.

**`open_diff` changes**: capture difft stdout as bytes, parse with ANSI crate, enter diff view loop. No pager subprocess. TUI does not leave the alternate screen.

**SPEC updates**: remove `pager` from Configuration; update Diff View and Navigation sections.

**Review cadence**: end of change.

## Plan

- [x] ADD `ansi-to-tui` dependency — confirm crate name and add to `Cargo.toml`
- [x] CHANGE `config.rs` — remove `pager` field and its validation
- [x] CHANGE `tui.rs` — replace `open_diff` pager pipe with difft capture + ANSI parse + diff view loop
- [x] CHANGE `main.rs` — remove `pager` argument from `tui.run` call; bump version to 0.2.0 in `Cargo.toml`
- [x] REVIEW `SPEC.md` — remove `pager` from Configuration; update Diff View and Navigation sections

## Conclusion

Replaced the external pager with an internal diff viewer in ratatui. difft output is captured at the current terminal width, ANSI SGR sequences are parsed by a new `src/ansi.rs` module (replacing the incompatible `ansi-to-tui` crate), and the result is rendered as a scrollable `Paragraph`. Terminal resize re-runs difft at the new width and resets scroll. The `pager` config field and its PATH validation were removed; `less` is no longer required. Version bumped to 0.2.0.
