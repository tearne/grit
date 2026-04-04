# Config Scaffold

## Intent

On first run, when no config file exists, grit creates one at the platform config path populated with all fields at their default values and inline comments explaining each option's purpose and valid values. This gives users a ready-to-edit starting point rather than an invisible config that must be discovered via documentation.

## Approach

`Config::load()` in `config.rs` currently falls through to `Config::default()` when the file is absent. The change adds a scaffold step before that fallback: create the config directory with `fs::create_dir_all`, write a template string to the config path, then return `Config::default()` as before. Errors during scaffold write propagate as startup errors — failure to write the config directory is an environmental problem worth surfacing.

The template is a `const &str` in `config.rs` with every field set to its default value and a comment block above each explaining its purpose and valid options, matching the SPEC's Configuration section exactly.

No SPEC changes needed — the scaffold behaviour is an implementation detail of the startup path, not a user-facing behavioural contract worth specifying.

Review cadence: at the end.

## Plan

- [x] UPDATE IMPL: add a `SCAFFOLD` const in `config.rs` — valid TOML with all fields at their defaults, each preceded by a comment stating its purpose and valid options
- [x] UPDATE IMPL: `Config::load()` in `config.rs` — when the file is absent, call `fs::create_dir_all` on the config directory, write `SCAFFOLD` to the config path, then return `Config::default()`
- [x] ADD TEST: parse `SCAFFOLD` and assert all fields match `Config::default()` values — verifies the template stays in sync with the defaults

## Conclusion

`SCAFFOLD` const added to `config.rs` with all fields at their defaults and explanatory comments. `Config::load()` now writes the scaffold to `~/.config/grit/config.toml` (creating the directory if needed) on first run. Test confirms the scaffold stays in sync with `Config::default()`. 28 tests passing.
