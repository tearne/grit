# Optional ref-b

## Intent

Allow `grit` to be invoked with a single ref, implicitly comparing it against the working tree. `grit <ref>` is shorthand for `grit <ref> .`, making the common case of reviewing uncommitted changes against a branch or commit faster to invoke.

## Approach

Make `ref_b` optional in the clap `Args` struct (`Option<String>`), defaulting to `"."` when absent. The default is applied before `validate_refs` so all existing validation and downstream logic is unchanged.

**Version bump**: patch — 0.3.4.

**Review cadence**: end of change.

## Plan

- [x] CHANGE `main.rs` — make `ref_b` an `Option<String>`, resolve to `"."` when absent
- [x] CHANGE `SPEC.md` — update invocation syntax and note the single-arg default
- [x] CHANGE `Cargo.toml` — bump version to 0.3.4

## Conclusion

Made `ref_b` optional in the clap `Args` struct, resolved to `"."` via an `Args::resolve` method before any downstream logic. All existing validation and session/worktree code is unchanged. Updated SPEC invocation syntax and bumped to 0.3.4.
