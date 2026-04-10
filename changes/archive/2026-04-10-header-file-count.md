# Header File Count

## Intent
The header row shows the two git refs being compared but gives no sense of review progress. Users should be able to see at a glance how many of the changed files they have reviewed, displayed as a fraction (reviewed / total) in the header.

## Approach
`title_bar_line` in `src/tui.rs` gains two new parameters: `reviewed: usize` and `total: usize`. When `refresh_pending` is false, the right-hand side becomes `  {reviewed} / {total}  grit v{version}  `. The refresh notification is unchanged.

At the call site (line 299), the counts are computed inline from `session.files` — `total` is `session.files.len()`, and `reviewed` is a count of files whose `state` matches `ReviewedStable | ReviewedDirty`. `ReviewState` is already `pub(crate)` so no visibility changes are needed; `is_reviewed` in `session.rs` remains private and the match is inlined at the call site.

## Plan
- [x] UPDATE IMPL `title_bar_line`: add `reviewed: usize` and `total: usize` parameters; update the non-refresh right-hand string to `  {reviewed} / {total}  grit v{version}  `
- [x] UPDATE IMPL call site in `render_checklist`: compute `reviewed` and `total` from `session.files` and pass them to `title_bar_line`

## Conclusion
Delivered. The outer `total` variable already existed in `render_checklist` and was captured by the closure, so no duplicate definition was needed at the call site.
