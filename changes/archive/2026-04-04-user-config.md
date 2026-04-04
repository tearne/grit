# User Config

## Intent

The config file moves from `.grit.toml` at the repository root to the user's platform config directory (`~/.config/grit/config.toml` on Linux/macOS following the XDG convention). This makes settings like `diff_tool` and `theme` user-wide preferences rather than per-project files that would need to be committed or gitignored. The tradeoff — no per-project overrides — is accepted for now.

An `auto_refresh` field is added (integer, seconds) with a default of 10 seconds, replacing the hardcoded 30-second interval.

A `preview_split` field is added (integer, percentage of terminal height given to the preview pane) with a default of 50, replacing the hardcoded 2/3 file list / 1/3 preview split.

## Approach

The `dirs` crate is added as a dependency. `Config::load` resolves the config path as `dirs::config_dir()/grit/config.toml`, bailing with a clear message if `config_dir()` returns `None`. If the file is absent the defaults apply as before; the directory is not created automatically.

Two new fields on `Config`:
- `auto_refresh: u64` — seconds between automatic refreshes, default 10; validated ≥ 1
- `preview_split: u8` — percentage of terminal height given to the preview pane, default 50; validated 1–99

`tui.run()` gains `auto_refresh: u64` and `preview_split: u8` parameters. The hardcoded `30` at line 104 of `tui.rs` is replaced by `auto_refresh`. `default_split` is replaced by an inline calculation: `(available_height * preview_split as u16 / 100).max(2)`.

The SPEC is updated: the config path changes to `~/.config/grit/config.toml` (with a note that this is the XDG default; the actual path follows the platform convention), and the example is extended to show `auto_refresh` and `preview_split`.

Review cadence: per-task.

## Plan

- [x] ADD IMPL: add `dirs` crate to `Cargo.toml`
- [x] UPDATE IMPL: `Config::load` — resolve path via `dirs::config_dir()/grit/config.toml`; bail with a clear message if `config_dir()` returns `None`
- [x] UPDATE IMPL: add `auto_refresh: u64` (default 10) and `preview_split: u8` (default 50) to `Config`; validate `auto_refresh >= 1` and `preview_split` in 1–99
- [x] UPDATE IMPL: `tui.run()` — add `auto_refresh: u64` and `preview_split: u8` parameters; replace hardcoded `30` with `auto_refresh`; replace `default_split(available_height)` with `(available_height * preview_split as u16 / 100).max(2)`
- [x] UPDATE IMPL: `main.rs` — pass `config.auto_refresh` and `config.preview_split` through to `tui.run()`
- [x] REMOVE IMPL: `default_split` function in `tui.rs` — no longer needed
- [x] ADD TEST: `Config` loads `auto_refresh` and `preview_split` from a toml string
- [x] ADD TEST: `Config` validation rejects `auto_refresh = 0`
- [x] ADD TEST: `Config` validation rejects `preview_split = 0` and `preview_split = 100`
- [x] UPDATE SPEC: update config file path to `~/.config/grit/config.toml` with XDG note; add `auto_refresh` and `preview_split` to the example; update the default split description from 2/3 to 50%

## Conclusion

Config moved from `.grit.toml` at the repo root to `~/.config/grit/config.toml` (platform-resolved via the `dirs` crate). `auto_refresh` and `preview_split` added to `Config` with defaults of 10s and 50%; `default_split` removed from `tui.rs`; version bumped to 0.5.2.
