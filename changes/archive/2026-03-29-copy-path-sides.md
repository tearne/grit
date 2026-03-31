# Copy Path — Both Sides

## Intent

Make it possible to copy the path for either side of the diff, not just the right side. The title bar is updated to make the two refs visually distinct and unambiguous, so users know which side they are copying.

## Approach

**Version bump**: patch — 0.3.1. (Targeting this as independent of the in-progress checklist-columns change; version will need reconciling if they land together.)

**Title bar**: remove `grit`, remove the reviewed count, and widen the visual separation between refs. New format:

```
  {ref_a}  →  {ref_b}
```

The arrow communicates directionality (old → new). Both refs are visually balanced with space between them.

**Copy-path bindings**:
- `y` — copy path for `ref_b` (right/new side) — preserves existing muscle memory
- `Y` (Shift-y) — copy path for `ref_a` (left/old side)

**Footer hint** — trimmed to remove redundant or self-evident entries:

```
 j/k navigate   r toggle reviewed   y/Y copy path (new/old)
```

Removed: `Enter fullscreen` (shown on divider), `=/- split` (moved to divider, right side), `q quit` (Esc/q/Ctrl-C all work).

**Divider** — label on the left, resize hint on the right, `─` fill between:

```
─── Preview · Enter to expand ──────────────── =/- resize ───
```

**Review cadence**: end of change.

## Plan

- [x] CHANGE `tui.rs` — update title bar format; add `Y` binding for ref_a path copy; update footer hint
- [x] CHANGE `Cargo.toml` — bump version to 0.3.1
- [x] REVIEW `SPEC.md` — update title bar description; update clipboard and navigation sections

## Conclusion

Removed `grit` and the reviewed count from the title bar; the header now shows `  {ref_a}  →  {ref_b}`. Added `Y` to copy the ref_a (old) path alongside the existing `y` for ref_b (new). Moved `=/- split` hint off the footer and onto the right side of the divider; trimmed footer to `j/k navigate   r toggle reviewed   y/Y copy path (new/old)`. Version bumped to 0.3.1.
