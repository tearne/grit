# Theme Diff Colors

## Intent

The diff preview and fullscreen diff view render `difft` output using its own ANSI colors, which are standard terminal green and red. These are visually harsh compared to the autumn theme's softer, warmer green (`#99BE70`) and red (`#F05E48`). The diff output colors should match the theme so the whole UI feels consistent.

## Approach

`ansi::parse` already converts every ANSI SGR code to a ratatui `Color`. Adding a remap at that point — replacing ANSI green and red with theme-specific colors — is sufficient to align the diff output with the rest of the UI.

Two new fields are added to `Theme`: `diff_added: Color` and `diff_deleted: Color`. In `ansi::parse` (and transitively `apply_sgr`), any foreground or background color that maps to a green variant (`Color::Green`, `Color::LightGreen`) is replaced with `diff_added`; any red variant (`Color::Red`, `Color::LightRed`) is replaced with `diff_deleted`. Both fg and bg are remapped so difft's word-level highlight backgrounds are also softened.

For the default theme: `diff_added = Color::Green`, `diff_deleted = Color::Red` (no visible change from current behaviour).
For the autumn theme: `diff_added = Color::Rgb(0x99, 0xBE, 0x70)`, `diff_deleted = Color::Rgb(0xF0, 0x5E, 0x48)` — matching `status_added`/`status_deleted`.

`ansi::parse` gains `diff_added: Color, diff_deleted: Color` parameters. `capture_diff` forwards them. `WorkItem` carries them so the background `DiffWorker` thread has everything it needs without sharing a reference to `Theme`. `work_items()` accepts `diff_added` and `diff_deleted` and stamps them onto each `WorkItem`.

Theme cycling already clears nothing — a `cached.clear()` and `worker.reset()` must be added to the `CycleTheme` branch so stale parsed text is discarded when the user presses `t`.

A Themes section is added to the SPEC describing the available themes (`default`, `autumn`), that a theme governs the full UI — checklist, status indicators, and diff output — and that diff tool colors (ANSI green/red) are remapped to match the active theme's palette.

Review cadence: per-task.

## Plan

- [ ] UPDATE IMPL: add `diff_added: Color` and `diff_deleted: Color` to `Theme`; set autumn values to `#99BE70`/`#F05E48`, default values to `Color::Green`/`Color::Red`
- [ ] UPDATE IMPL: `ansi::parse` — add `diff_added: Color, diff_deleted: Color` parameters; remap green and red variants (fg and bg) in `apply_sgr`
- [ ] UPDATE IMPL: `capture_diff` — accept and forward `diff_added`/`diff_deleted` to `ansi::parse`
- [ ] UPDATE IMPL: `WorkItem` — add `diff_added: Color, diff_deleted: Color` fields; update `work_items()` to accept and stamp them
- [ ] UPDATE IMPL: `CycleTheme` branch in `tui.rs` — add `cached.clear()` and `worker.reset()` so stale parsed text is discarded on theme change
- [ ] ADD TEST: `ansi::parse` remaps ANSI green fg to the supplied `diff_added` color
- [ ] ADD TEST: `ansi::parse` remaps ANSI red fg to the supplied `diff_deleted` color
- [ ] ADD TEST: `ansi::parse` remaps ANSI green bg to the supplied `diff_added` color
- [ ] ADD TEST: `ansi::parse` remaps ANSI red bg to the supplied `diff_deleted` color
- [ ] UPDATE SPEC: add a Themes section describing the available themes and that the active theme governs the full UI including diff output colors

## Conclusion
