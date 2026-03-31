# Async Diff

## Intent

Diff capture is currently synchronous — while `difft` runs, the TUI is completely unresponsive. For large or complex diffs this causes a noticeable hang. Diff capture should run in the background so the UI remains responsive at all times. The preview pane shows a static "pending" indicator when a diff has not yet started, and a braille spinner while capture is in progress.

## Approach

One persistent worker thread runs for the lifetime of the TUI, processing diffs sequentially from a shared queue. Results are cached on the main thread; the preview shows a braille spinner until a file's diff is cached.

**Worker thread:**

A `DiffWorker` is created at the start of `run`. It owns:
- `queue: Arc<Mutex<VecDeque<WorkItem>>>` — shared with the main thread; each item holds the file index, both paths, and terminal width
- `result_rx: Receiver<(usize, Text<'static>)>` — receives completed diffs

The worker loop pops from the front of the queue, runs `capture_diff`, and sends `(index, text)` back. When the queue is empty it sleeps briefly (50 ms) before checking again.

**Main thread:**

- `cached: HashMap<usize, Text<'static>>` — completed diffs keyed by file index
- Each tick: drain `result_rx` with `try_recv` in a loop, inserting into `cached`
- Preview: `cached.get(selected)` → `Ready`; otherwise → `Loading` (spinner)
- On selection change: if not cached, move that index to the front of the queue (remove from its current position, push front)
- On resize: clear `cached`, rebuild the queue with all file indices at the new width

The worker cannot be interrupted mid-diff, so if a slow diff is in progress the selected file waits behind it. The UI remains responsive throughout.

**Rendering:**

- `Loading` — braille spinner frame (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`) followed by `  analysing…`
- `Ready` — diff text as today

**Poll timeout:** 100 ms while any diff is still outstanding (spinner needs to animate); 250 ms once all diffs are cached.

**Scope:** checklist preview pane only. The fullscreen diff view (`run_diff_view`) is synchronous and out of scope.

**Version bump:** patch — 0.3.5.

**Review cadence:** end of change.

## Plan

- [x] ADD `tui.rs` — `DiffWorker` struct and worker thread loop
- [x] UPDATE `tui.rs` — replace `preview` / `previewed_index` with `cached: HashMap` and `spinner_frame`; drain results each tick
- [x] UPDATE `tui.rs` — on selection change, reprioritise queue; on resize, clear cache and rebuild queue
- [x] UPDATE `tui.rs` — render `Loading` (spinner) and `Ready` states; poll timeout 100 ms while outstanding diffs remain
- [x] CHANGE `Cargo.toml` — bump version to 0.3.5

## Conclusion

Replaced the synchronous per-selection diff capture with a single persistent worker thread. `DiffWorker` owns a shared `Arc<Mutex<VecDeque<WorkItem>>>` queue and an `mpsc` result channel. The main loop drains completed results into a `HashMap` cache each tick, reprioritises the selected file on navigation changes, and shows a braille spinner in the preview pane until the file's diff is cached. On resize, the cache is cleared and the queue rebuilt with the new width. Poll timeout is 100 ms while diffs are outstanding, 250 ms once all are cached.
