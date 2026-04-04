# Session ID Dedup

## Intent

The session ID format is computed in two places: once in `session.rs` (private) and again in `main.rs` for the worktrees directory path. A change to the format would require two edits. The private helper should be exposed and reused.

## Approach

Change `session_id` in `session.rs` from `fn` to `pub(crate) fn`. Replace the duplicated `format!` in `main.rs` with a call to `session::session_id(&ref_a, &ref_b)`.

Review cadence: at the end.

## Plan

- [x] UPDATE IMPL: `session_id` in `session.rs` — change to `pub(crate) fn`
- [x] UPDATE IMPL: `main.rs` — replace the duplicated `format!("{}__{}", ...)` with `session::session_id(&ref_a, &ref_b)`

## Conclusion

Delivered as planned. No surprises.
