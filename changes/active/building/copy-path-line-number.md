# Copy Path Line Number

## Intent
When copying a file path with y/Y, the line number reflects the scroll position within the diff view rather than the corresponding line number in the actual file. The copied path should reference the file line number at the top of the visible preview.

## Approach

`difft` (the default diff tool) embeds file line numbers in each rendered output line: the old-file line number appears at the left margin, the new-file line number appears at approximately the midpoint of the terminal width. The line numbers are already present as text content in the `Text<'static>` stored in the cache — they can be extracted without re-running the diff.

Add a function `diff_line_numbers(text: &Text, scroll: usize, width: u16) -> (Option<u32>, Option<u32>)` that:
1. Reconstructs the plain text of `text.lines[scroll]` by concatenating span contents
2. Parses the leading integer for the old (left-column) line number
3. Parses a leading integer at approximately column `width / 2` for the new (right-column) line number
4. Returns `(old_line, new_line)` — `None` for header or hunk-separator lines that carry no file line number

`CopyPathOld` uses the old line; `CopyPathNew` uses the new line. Both fall back to `preview_scroll as u32 + 1` when the result is `None`. `width` is already in scope in the event loop.

Update `SPEC.md` to reflect that the line number is derived from the diff output, with scroll+1 as fallback.

Review cadence: single review at completion.

## Plan
- [x] ADD `tui.rs`: `diff_line_numbers(text: &Text, scroll: usize, width: u16) -> (Option<u32>, Option<u32>)` — reconstruct plain text of the line at `scroll`, parse leading integer for old line (left column), parse leading integer at ~`width / 2` for new line (right column)
- [x] UPDATE `tui.rs`: `CopyPathOld` and `CopyPathNew` handlers call `diff_line_numbers` and fall back to `preview_scroll as u32 + 1` when `None`
- [x] UPDATE `SPEC.md`: replace "The line number always reflects the current preview scroll position (`scroll + 1`)" with behaviour derived from diff output with scroll+1 fallback
