# Chrome Redesign

## Intent
The header, footer, and preview separator carry too much — version, file counts, and all keybindings share space with no clear hierarchy. The redesign gives each bar a single, coherent role: the header tracks review progress, the footer identifies the application and signals that help is available, and the separator surfaces the controls most relevant to the preview pane.

## Approach
All changes are in `src/tui.rs`.

**Header** (`title_bar_line`): Remove the version string from the right side. Right becomes `  N / M  ` (reviewed / total files). The `grit v{}` format string is removed here and relocated to the footer.

**Footer** (`render_checklist`): Replace the full keybinding hint string with a two-part line: left-aligned `grit v{CARGO_PKG_VERSION}`, right-aligned `? for help`, with padding between. The `?` key is not wired up in this change — the footer text is forward-looking.

**Separator** (non-fullscreen branch in `render_checklist`): Replace the current `─── Preview · Enter to expand` / `-/= resize ───` split with three labelled sections across the bar:
- Left anchor: `─── Y:old ` (copy-old-path hint)
- Centre: ` -/= ↓/↑ ` (resize hint)
- Right anchor: ` y:new ───` (copy-new-path hint)

The fill `─` characters are distributed so the centre label sits in the middle of the terminal width. The fullscreen branch is unchanged.

The `title_bar_line` function signature and call site are unchanged — only the `right` string constructed inside it changes.

**Help modal**: A `show_help: bool` flag is added to the `run` loop. Pressing `?` toggles it via a new `ChecklistControl::ToggleHelp` variant. When the modal is open, the event loop intercepts all key events before reaching `handle_checklist_key`: any key closes the modal and returns `Continue`; no key is passed through. The modal is rendered as a last step inside `terminal.draw` — a centred fixed-size `Block` with a title, cleared with ratatui's `Clear` widget before drawing, then a `Paragraph` listing all keybindings. Modal width is 44, height is sized to the content. The keybindings shown are those currently in the footer hint string plus theme cycling (`t`).

## Plan
- [x] ADD `ChecklistControl::ToggleHelp` variant to the enum
- [x] UPDATE `handle_checklist_key`: wire `?` to `ToggleHelp`
- [x] ADD `show_help: bool` to the `run` loop state; intercept all key events when true, toggling off and returning `Continue`
- [x] UPDATE `run` loop: handle `ChecklistControl::ToggleHelp` to flip `show_help`
- [x] UPDATE `title_bar_line`: right side becomes `  N / M  ` (drop version string)
- [x] UPDATE separator non-fullscreen branch: three-part label — `─── Y:old `, centred ` -/= ↓/↑ `, ` y:new ───`
- [x] UPDATE footer: left-aligned `grit v{CARGO_PKG_VERSION}`, right-aligned `? for help`, padding between
- [x] ADD `render_help_modal` function: centred overlay using `Clear` + `Block` + `Paragraph` listing all keybindings
- [x] UPDATE `render_checklist`: accept `show_help` param; call `render_help_modal` when true

## Conclusion
All tasks delivered as planned. The header now shows only the reviewed/total progress count; the footer shows `grit v{version}` left and `? for help` right; the separator carries three anchored labels for copy-path and resize hints; and pressing `?` opens a centred help overlay listing all keybindings. Version bumped `0.5.16 → 0.5.17`.
