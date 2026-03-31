# Show Version

## Intent

Expose the crate version in two places: via a `--version` CLI flag, and in the TUI title bar. Users should be able to check which version they have installed without launching a full session, and see it at a glance while using the tool.

## Approach

**Version bump**: patch — 0.3.3. (Approach was written at 0.2.1; reconciled after copy-path-sides landed at 0.3.2.)

**CLI flag**: add `#[command(version)]` to the clap `Args` struct. Clap reads `CARGO_PKG_VERSION` at compile time and wires `--version` / `-V` automatically.

**TUI title bar**: the current format is `  {ref_a}  →  {ref_b}`. Name and version are appended flush to the right edge. During rendering the area width is known, so the gap between the two sides is filled with spaces:

```
  {ref_a}  →  {ref_b}               grit v0.3.3
```

The version string is embedded at compile time via `env!("CARGO_PKG_VERSION")`. Two spaces of padding are kept inside the right edge.

**Review cadence**: end of change.

## Plan

- [x] CHANGE `main.rs` — add `#[command(version)]` to `Args`
- [x] CHANGE `tui.rs` — right-align `grit v{version}` in the checklist title bar using `env!("CARGO_PKG_VERSION")` and the render area width
- [x] CHANGE `Cargo.toml` — bump version to 0.3.3
- [x] REVIEW `SPEC.md` — document `--version` flag; note version appears in title bar

## Conclusion

Added `#[command(version)]` to the clap `Args` struct, giving `grit --version` / `-V` for free. Updated `title_bar_text` in `tui.rs` to append `grit v{version}` flush to the right edge of the title bar using `env!("CARGO_PKG_VERSION")`. Bumped `Cargo.toml` to `0.3.3`. Updated `SPEC.md` to document both.
