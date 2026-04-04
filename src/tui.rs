use std::collections::{HashMap, VecDeque};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use color_eyre::eyre::Result;
use ratatui::crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, List, ListItem, ListState, Paragraph},
    Terminal,
};

use crate::checklist::Checklist;
use crate::open_command;
use crate::theme::Theme;

const BRAILLE_FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const NOTIFICATION_TTL: Duration = Duration::from_secs(3);

pub(crate) struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl Tui {
    pub(crate) fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        Ok(Tui { terminal })
    }

    pub(crate) fn run(
        &mut self,
        checklist: &mut Checklist,
        repo_root: &Path,
        diff_tool: &str,
        initial_theme: Theme,
        worktree_a: &Path,
        worktree_b: &Path,
        auto_refresh: u64,
        preview_split: u8,
    ) -> Result<()> {
        let mut theme = initial_theme;
        let mut available_height = self.terminal.size()?.height.saturating_sub(3);
        let mut split_row = (available_height * preview_split as u16 / 100).max(2);
        let mut dragging = false;
        let mut list_state = ListState::default();
        let mut cached: HashMap<usize, Text<'static>> = HashMap::new();
        let mut spinner_frame: u8 = 0;
        let mut last_selected: Option<usize> = None;
        let mut preview_scroll: usize = 0;
        let mut notification: Option<(String, std::time::Instant)> = None;
        let mut last_refresh = std::time::Instant::now();
        let mut saved_split: Option<u16> = None;
        let mut refresh_pending = false;
        let mut view_mode = ViewMode::Tree;

        let mut file_paths = build_file_paths(&checklist.session.files, worktree_a, worktree_b);

        let mut width = self.terminal.size()?.width.saturating_sub(1);
        let worker = DiffWorker::start(work_items(&file_paths, &diff_tool, width, theme.diff_added, theme.diff_deleted, theme.diff_unmatched));

        loop {
            let total = checklist.session.files.len();

            while let Ok((index, text)) = worker.result_rx.try_recv() {
                cached.insert(index, text);
            }

            if total > 0 && last_selected != Some(checklist.selected) {
                last_selected = Some(checklist.selected);
                preview_scroll = 0;
                notification = None;
                if !cached.contains_key(&checklist.selected) {
                    worker.prioritize(checklist.selected);
                }
            }

            let all_done = total == 0 || cached.len() == total;
            if !all_done {
                spinner_frame = (spinner_frame + 1) % BRAILLE_FRAMES.len() as u8;
            }

            if let Some((_, instant)) = &notification {
                if instant.elapsed() >= NOTIFICATION_TTL {
                    notification = None;
                }
            }

            let preview = cached.get(&checklist.selected);
            let active_notif = active_notification(&notification);
            self.render_checklist(checklist, &mut list_state, preview, spinner_frame, preview_scroll, split_row, active_notif, &theme, saved_split.is_some(), refresh_pending, view_mode)?;

            if last_refresh.elapsed() >= std::time::Duration::from_secs(auto_refresh) {
                if checklist.session.has_pending_changes(repo_root)? {
                    refresh_pending = true;
                }
                last_refresh = std::time::Instant::now();
            }

            match self.next_checklist_event(checklist, split_row, &mut dragging, !all_done, saved_split.is_some(), width)? {
                ChecklistControl::Continue => {}
                ChecklistControl::Save => checklist.session.save(repo_root)?,
                ChecklistControl::Refresh => {
                    refresh_pending = false;
                    let selected_path = checklist.selected_file().map(|f| f.path.clone());
                    checklist.refresh(repo_root)?;
                    checklist.session.save(repo_root)?;
                    last_refresh = std::time::Instant::now();
                    cached.clear();
                    // Restore selection to the same file by path. If found, preserve
                    // preview_scroll by keeping last_selected in sync with the new index.
                    // If the file was removed, fall back to scroll reset via last_selected = None.
                    let same_file_idx = selected_path.as_ref().and_then(|path| {
                        checklist.session.files.iter().position(|f| &f.path == path)
                    });
                    if let Some(idx) = same_file_idx {
                        checklist.selected = idx;
                        last_selected = Some(idx);
                        worker.prioritize(idx);
                    } else {
                        last_selected = None;
                    }
                    file_paths = build_file_paths(&checklist.session.files, worktree_a, worktree_b);
                    worker.reset(work_items(&file_paths, &diff_tool, width, theme.diff_added, theme.diff_deleted, theme.diff_unmatched));
                }
                ChecklistControl::CycleTheme => {
                    theme = theme.next();
                    cached.clear();
                    last_selected = None;
                    worker.reset(work_items(&file_paths, &diff_tool, width, theme.diff_added, theme.diff_deleted, theme.diff_unmatched));
                }
                ChecklistControl::ToggleFullscreen => {
                    match saved_split {
                        None => { saved_split = Some(split_row); split_row = 0; }
                        Some(restored) => { split_row = restored; saved_split = None; }
                    }
                }
                ChecklistControl::CopyPathNew => {
                    if let Some(file) = checklist.selected_file() {
                        let line_num = cached.get(&checklist.selected)
                            .and_then(|text| diff_line_numbers(text, preview_scroll, width).1)
                            .unwrap_or((preview_scroll + 1) as u32);
                        let mut msg = open_command::copy(&worktree_b.join(&file.path), line_num);
                        if crate::worktree::is_managed(worktree_b, repo_root) {
                            msg.push_str("  ⚠ grit-managed worktree — edits will be lost on exit");
                        }
                        notification = Some((msg, std::time::Instant::now()));
                    }
                }
                ChecklistControl::CopyPathOld => {
                    if let Some(file) = checklist.selected_file() {
                        let line_num = cached.get(&checklist.selected)
                            .and_then(|text| diff_line_numbers(text, preview_scroll, width).0)
                            .unwrap_or((preview_scroll + 1) as u32);
                        let mut msg = open_command::copy(&worktree_a.join(&file.path), line_num);
                        if crate::worktree::is_managed(worktree_a, repo_root) {
                            msg.push_str("  ⚠ grit-managed worktree — edits will be lost on exit");
                        }
                        notification = Some((msg, std::time::Instant::now()));
                    }
                }
                ChecklistControl::Quit => {
                    checklist.session.save(repo_root)?;
                    return Ok(());
                }
                ChecklistControl::AdjustSplit(delta) => {
                    if saved_split.is_none() {
                        let max = available_height.saturating_sub(2);
                        split_row = (split_row as i32 + delta).clamp(2, max as i32) as u16;
                    }
                }
                ChecklistControl::ScrollPreview(delta) => {
                    if let Some(text) = cached.get(&checklist.selected) {
                        let total_lines = text.lines.len();
                        let preview_height = available_height.saturating_sub(split_row) as usize;
                        let max = if total_lines <= preview_height {
                            0
                        } else {
                            total_lines - preview_height + preview_height / 4
                        };
                        preview_scroll = (preview_scroll as i32 + delta).clamp(0, max as i32) as usize;
                    }
                }
                ChecklistControl::UpdateDrag(row) => {
                    // row is the terminal row the cursor is on; title bar occupies row 0
                    let new_split = row.saturating_sub(1);
                    let max = available_height.saturating_sub(2);
                    split_row = new_split.clamp(2, max);
                }
                ChecklistControl::Resize => {
                    let size = self.terminal.size()?;
                    let new_available = size.height.saturating_sub(3);
                    if available_height > 0 {
                        split_row = ((split_row as u32 * new_available as u32)
                            / available_height as u32)
                            .clamp(2, new_available.saturating_sub(2) as u32)
                            as u16;
                    }
                    available_height = new_available;
                    width = size.width.saturating_sub(1);
                    cached.clear();
                    last_selected = None;
                    worker.reset(work_items(&file_paths, &diff_tool, width, theme.diff_added, theme.diff_deleted, theme.diff_unmatched));
                }
                ChecklistControl::ToggleView => {
                    view_mode = match view_mode {
                        ViewMode::Flat => ViewMode::Tree,
                        ViewMode::Tree => ViewMode::Flat,
                    };
                    last_selected = None;
                }
                ChecklistControl::SelectVisualRow(row) => {
                    let absolute = list_state.offset() + row;
                    match view_mode {
                        ViewMode::Flat => checklist.select_index(absolute),
                        ViewMode::Tree => {
                            let tree = build_tree(&checklist.session.files);
                            if let Some(TreeRow::File { file_index, .. }) = tree.get(absolute) {
                                checklist.selected = *file_index;
                                last_selected = None;
                            }
                        }
                    }
                }
                ChecklistControl::SelectAdjacentFile(delta) => {
                    match view_mode {
                        ViewMode::Flat => {
                            if delta > 0 { checklist.select_next(); } else { checklist.select_prev(); }
                        }
                        ViewMode::Tree => {
                            let tree = build_tree(&checklist.session.files);
                            let file_rows: Vec<usize> = tree.iter()
                                .filter_map(|row| {
                                    if let TreeRow::File { file_index, .. } = row { Some(*file_index) } else { None }
                                })
                                .collect();
                            if let Some(pos) = file_rows.iter().position(|&fi| fi == checklist.selected) {
                                let new_pos = if delta > 0 {
                                    (pos + 1).min(file_rows.len().saturating_sub(1))
                                } else {
                                    pos.saturating_sub(1)
                                };
                                checklist.selected = file_rows[new_pos];
                            }
                        }
                    }
                }
            }
        }
    }

    fn render_checklist(
        &mut self,
        checklist: &Checklist,
        list_state: &mut ListState,
        preview: Option<&Text<'static>>,
        spinner_frame: u8,
        preview_scroll: usize,
        split_row: u16,
        notification: Option<&str>,
        theme: &Theme,
        fullscreen: bool,
        refresh_pending: bool,
        view_mode: ViewMode,
    ) -> Result<()> {
        let session = &checklist.session;
        let total = session.files.len();
        let preview = preview.cloned();

        self.terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(Block::default().style(Style::default().bg(theme.background)), area);
            let vertical = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(split_row),
                    Constraint::Length(1),
                    Constraint::Min(0),
                    Constraint::Length(1),
                ])
                .split(area);

            let title_bar = Paragraph::new(title_bar_line(&session.ref_a, &session.ref_b, vertical[0].width, refresh_pending))
                .style(theme.title_bar);
            frame.render_widget(title_bar, vertical[0]);

            // File list with scroll-indicator column.
            let list_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(vertical[1]);

            let max_add = session.files.iter().map(|f| f.additions).max().unwrap_or(0);
            let max_del = session.files.iter().map(|f| f.deletions).max().unwrap_or(0);
            let count_width = format!("+{max_add}").len().max(format!("-{max_del}").len());

            let (items, selected_visual_row): (Vec<ListItem>, Option<usize>) = match view_mode {
                ViewMode::Flat => {
                    let items = session.files.iter().map(|f| {
                        flat_file_item(f, count_width, theme)
                    }).collect();
                    (items, if total > 0 { Some(checklist.selected) } else { None })
                }
                ViewMode::Tree => {
                    let tree = build_tree(&session.files);
                    // Measure the longest prefix+filename so all stats share the same x-position.
                    let max_left_width = tree.iter().filter_map(|row| {
                        if let TreeRow::File { prefix, file_index } = row {
                            let f = &session.files[*file_index];
                            let filename = f.path.file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| f.path.display().to_string());
                            Some(format!("{}{}", prefix, filename).chars().count())
                        } else {
                            None
                        }
                    }).max().unwrap_or(0);
                    let mut selected_row = None;
                    let items = tree.iter().enumerate().map(|(row_idx, row)| {
                        match row {
                            TreeRow::Dir { prefix, label } => {
                                ListItem::new(Line::from(vec![
                                    Span::raw(prefix.clone()),
                                    Span::styled(label.clone(), Style::default().add_modifier(Modifier::DIM)),
                                ]))
                            }
                            TreeRow::File { prefix, file_index } => {
                                if *file_index == checklist.selected {
                                    selected_row = Some(row_idx);
                                }
                                let f = &session.files[*file_index];
                                let filename = f.path.file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| f.path.display().to_string());
                                tree_file_item(prefix, &filename, f, count_width, max_left_width, list_chunks[0].width, theme)
                            }
                        }
                    }).collect();
                    (items, selected_row)
                }
            };

            list_state.select(selected_visual_row);
            let item_count = items.len();
            let list = List::new(items).highlight_style(theme.list_highlight);
            frame.render_stateful_widget(list, list_chunks[0], list_state);

            let list_height = list_chunks[0].height as usize;
            let offset = list_state.offset();
            let list_indicator = scrollbar(item_count, offset, list_height, list_chunks[1].height);
            frame.render_widget(Paragraph::new(list_indicator), list_chunks[1]);

            // Divider — label and hint vary by fullscreen state.
            let (left, right) = if fullscreen {
                let path = checklist.selected_file()
                    .map(|f| f.path.display().to_string())
                    .unwrap_or_default();
                (format!("─── {path} · Enter to restore "), " ───".to_string())
            } else {
                ("─── Preview · Enter to expand ".to_string(), " -/= resize ───".to_string())
            };
            let fill_len = (area.width as usize)
                .saturating_sub(left.chars().count() + right.chars().count());
            let divider = Paragraph::new(format!("{left}{}{right}", "─".repeat(fill_len)))
                .style(theme.divider);
            frame.render_widget(divider, vertical[2]);

            // Preview with scroll-indicator column.
            let preview_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(vertical[3]);

            if let Some(text) = &preview {
                frame.render_widget(
                    Paragraph::new(text.clone()).scroll((preview_scroll as u16, 0)),
                    preview_chunks[0],
                );
                let preview_height = preview_chunks[0].height as usize;
                let preview_indicator = scrollbar(text.lines.len(), preview_scroll, preview_height, preview_chunks[1].height);
                frame.render_widget(Paragraph::new(preview_indicator), preview_chunks[1]);
            } else if total > 0 {
                let spinner = BRAILLE_FRAMES[spinner_frame as usize];
                frame.render_widget(
                    Paragraph::new(format!("  {spinner}  analysing…")),
                    preview_chunks[0],
                );
            }

            let footer_text = notification.unwrap_or(
                " u/i navigate   j/k scroll preview   space toggle reviewed   r refresh   y/Y copy path (new/old)   v tree/flat",
            );
            let footer_style = if notification.is_some() {
                theme.footer_notification
            } else {
                theme.footer
            };
            frame.render_widget(
                Paragraph::new(format!(" {footer_text}")).style(footer_style),
                vertical[4],
            );
        })?;

        Ok(())
    }

    fn next_checklist_event(
        &self,
        checklist: &mut Checklist,
        split_row: u16,
        dragging: &mut bool,
        loading: bool,
        fullscreen: bool,
        width: u16,
    ) -> Result<ChecklistControl> {
        let timeout = if loading {
            Duration::from_millis(100)
        } else {
            Duration::from_millis(250)
        };
        if !event::poll(timeout)? {
            return Ok(ChecklistControl::Continue);
        }
        match event::read()? {
            Event::Key(key) => Ok(handle_checklist_key(key, checklist)),
            Event::Mouse(mouse) => Ok(handle_mouse(mouse, split_row, dragging, fullscreen, width)),
            Event::Resize(_, _) => Ok(ChecklistControl::Resize),
            _ => Ok(ChecklistControl::Continue),
        }
    }

}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        let _ = self.terminal.show_cursor();
    }
}

struct WorkItem {
    index: usize,
    path_a: PathBuf,
    path_b: PathBuf,
    diff_tool: String,
    width: u16,
    diff_added: Color,
    diff_deleted: Color,
    diff_unmatched: Color,
}

struct DiffWorker {
    queue: Arc<Mutex<VecDeque<WorkItem>>>,
    result_rx: std::sync::mpsc::Receiver<(usize, Text<'static>)>,
}

impl DiffWorker {
    fn start(initial_queue: VecDeque<WorkItem>) -> Self {
        let queue = Arc::new(Mutex::new(initial_queue));
        let (tx, rx) = std::sync::mpsc::channel();
        let worker_queue = Arc::clone(&queue);
        std::thread::spawn(move || {
            loop {
                let item = worker_queue.lock().expect("diff queue lock poisoned").pop_front();
                match item {
                    Some(item) => {
                        let text = capture_diff(&item.diff_tool, &item.path_a, &item.path_b, item.width, item.diff_added, item.diff_deleted, item.diff_unmatched)
                            .unwrap_or_default();
                        if tx.send((item.index, text)).is_err() {
                            break;
                        }
                    }
                    None => std::thread::sleep(Duration::from_millis(50)),
                }
            }
        });
        DiffWorker { queue, result_rx: rx }
    }

    fn prioritize(&self, index: usize) {
        let mut q = self.queue.lock().expect("diff queue lock poisoned");
        if let Some(pos) = q.iter().position(|w| w.index == index) {
            let item = q.remove(pos).expect("position was valid");
            q.push_front(item);
        }
    }

    fn reset(&self, items: VecDeque<WorkItem>) {
        *self.queue.lock().expect("diff queue lock poisoned") = items;
    }
}

fn work_items(file_paths: &[(PathBuf, PathBuf)], diff_tool: &str, width: u16, diff_added: Color, diff_deleted: Color, diff_unmatched: Color) -> VecDeque<WorkItem> {
    file_paths
        .iter()
        .enumerate()
        .map(|(index, (path_a, path_b))| WorkItem {
            index,
            path_a: path_a.clone(),
            path_b: path_b.clone(),
            diff_tool: diff_tool.to_string(),
            width,
            diff_added,
            diff_deleted,
            diff_unmatched,
        })
        .collect()
}

enum TreeRow {
    Dir { prefix: String, label: String },
    File { prefix: String, file_index: usize },
}

fn build_tree(files: &[crate::session::FileEntry]) -> Vec<TreeRow> {
    // Build a virtual directory tree from the flat file list, then walk it
    // into a sequence of TreeRow values using box-drawing prefixes.
    // Directories with a single directory child are path-compressed.

    struct DirNode {
        // label used for display — may span multiple path components after compression
        label: String,
        children_dirs: Vec<DirNode>,
        children_files: Vec<usize>, // indices into `files`
    }

    fn insert(root: &mut DirNode, components: &[&str], file_index: usize) {
        if components.is_empty() {
            root.children_files.push(file_index);
            return;
        }
        let name = components[0];
        if let Some(child) = root.children_dirs.iter_mut().find(|d| d.label == name) {
            insert(child, &components[1..], file_index);
        } else {
            let mut child = DirNode { label: name.to_string(), children_dirs: vec![], children_files: vec![] };
            insert(&mut child, &components[1..], file_index);
            root.children_dirs.push(child);
        }
    }

    fn compress(node: &mut DirNode) {
        for child in &mut node.children_dirs {
            compress(child);
        }
        // Merge single-dir-child nodes upward (path compression)
        loop {
            if node.children_files.is_empty() && node.children_dirs.len() == 1 {
                let only = node.children_dirs.remove(0);
                node.label = format!("{}/{}", node.label, only.label);
                node.children_dirs = only.children_dirs;
                node.children_files = only.children_files;
            } else {
                break;
            }
        }
    }

    fn walk_children(node: &DirNode, rows: &mut Vec<TreeRow>, prefix: &str) {
        let dir_count = node.children_dirs.len();
        let file_count = node.children_files.len();
        let total = dir_count + file_count;

        for (i, dir) in node.children_dirs.iter().enumerate() {
            let last = i == total - 1 && file_count == 0;
            let connector = if last { "└ " } else { "├ " };
            rows.push(TreeRow::Dir { prefix: format!("{}{}", prefix, connector), label: format!("{}/", dir.label) });
            let child_prefix = format!("{}{}", prefix, if last { "  " } else { "│ " });
            walk_children(dir, rows, &child_prefix);
        }
        for (i, &file_index) in node.children_files.iter().enumerate() {
            let last = i == file_count - 1;
            let connector = if last { "└ " } else { "├ " };
            rows.push(TreeRow::File { prefix: format!("{}{}", prefix, connector), file_index });
        }
    }

    let mut root = DirNode { label: String::new(), children_dirs: vec![], children_files: vec![] };

    for (i, file) in files.iter().enumerate() {
        let components: Vec<&str> = file.path.components()
            .map(|c| c.as_os_str().to_str().expect("valid UTF-8 path"))
            .collect();
        if components.len() == 1 {
            root.children_files.push(i);
        } else {
            let dirs = &components[..components.len() - 1];
            insert(&mut root, dirs, i);
        }
    }

    for dir in &mut root.children_dirs {
        compress(dir);
    }

    let mut rows = Vec::new();
    let dir_count = root.children_dirs.len();
    let file_count = root.children_files.len();
    let total = dir_count + file_count;

    for (i, dir) in root.children_dirs.iter().enumerate() {
        let last = i == total - 1 && file_count == 0;
        let connector = if last { "└ " } else { "├ " };
        rows.push(TreeRow::Dir { prefix: connector.to_string(), label: format!("{}/", dir.label) });
        let child_prefix = if last { "  " } else { "│ " };
        walk_children(dir, &mut rows, child_prefix);
    }
    for (i, &file_index) in root.children_files.iter().enumerate() {
        let last = i == file_count - 1;
        let connector = if last { "└ " } else { "├ " };
        rows.push(TreeRow::File { prefix: connector.to_string(), file_index });
    }

    rows
}

enum ChecklistControl {
    Continue,
    Save,
    Refresh,
    CycleTheme,
    ToggleFullscreen,
    ToggleView,
    CopyPathNew,
    CopyPathOld,
    Quit,
    AdjustSplit(i32),
    ScrollPreview(i32),
    UpdateDrag(u16),
    SelectVisualRow(usize),
    SelectAdjacentFile(i32),
    Resize,
}

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Flat,
    Tree,
}

fn title_bar_line(ref_a: &str, ref_b: &str, width: u16, refresh_pending: bool) -> Line<'static> {
    let left = format!("  {}  →  {}", ref_a, ref_b);
    let right = if refresh_pending {
        "  Changes detected — press r to refresh  ".to_string()
    } else {
        format!("grit v{}  ", env!("CARGO_PKG_VERSION"))
    };
    let padding = (width as usize).saturating_sub(left.len() + right.len());
    let padded_left = format!("{}{}", left, " ".repeat(padding));
    if refresh_pending {
        Line::from(vec![
            Span::raw(padded_left),
            Span::styled(right, Style::default().bg(Color::Rgb(160, 40, 40)).fg(Color::White).add_modifier(Modifier::BOLD)),
        ])
    } else {
        Line::from(padded_left + &right)
    }
}

fn scrollbar(total: usize, offset: usize, visible: usize, height: u16) -> Text<'static> {
    let h = height as usize;
    if h == 0 || total <= visible {
        return Text::default();
    }
    let thumb_h = (visible * h / total).max(1);
    let thumb_top = if total <= visible {
        0
    } else {
        offset * (h - thumb_h) / (total - visible)
    };
    let lines: Vec<Line<'static>> = (0..h)
        .map(|i| {
            if i >= thumb_top && i < thumb_top + thumb_h {
                Line::from("█")
            } else {
                Line::from(" ")
            }
        })
        .collect();
    Text::from(lines)
}

fn count_style(count: u32, fg: Color, heavy_bg: Color) -> Style {
    match count {
        0         => Style::default().fg(fg).add_modifier(Modifier::DIM),
        1..=99    => Style::default().fg(fg),
        100..=999 => Style::default().fg(fg).add_modifier(Modifier::BOLD),
        _         => Style::default().fg(fg).bg(heavy_bg).add_modifier(Modifier::BOLD),
    }
}

fn flat_file_item(f: &crate::session::FileEntry, count_width: usize, theme: &Theme) -> ListItem<'static> {
    let checkbox = match f.state {
        crate::session::ReviewState::Unreviewed => "[ ]",
        crate::session::ReviewState::ReviewedStable => "[x]",
        crate::session::ReviewState::ReviewedDirty => "[~]",
    };
    let (status_char, status_style) = file_status_display(f.status, theme);
    let add_str = format!("+{}", f.additions);
    let del_str = format!("-{}", f.deletions);
    let mut spans = vec![
        Span::raw(format!(" {checkbox} ")),
        Span::styled(status_char, status_style),
        Span::raw("  "),
        Span::styled(format!("{:>count_width$}", add_str), count_style(f.additions, theme.count_positive, theme.count_positive_bg)),
        Span::raw("  "),
        Span::styled(format!("{:>count_width$}", del_str), count_style(f.deletions, theme.count_negative, theme.count_negative_bg)),
        Span::raw(format!("  {}", f.path.display())),
    ];
    if let crate::git::FileStatus::Moved = f.status {
        if let Some(old) = &f.old_path {
            spans.push(Span::styled(
                format!("  (was: {})", old.display()),
                Style::default().add_modifier(Modifier::DIM),
            ));
        }
    }
    ListItem::new(Line::from(spans))
}

fn tree_file_item(prefix: &str, filename: &str, f: &crate::session::FileEntry, count_width: usize, max_left_width: usize, row_width: u16, theme: &Theme) -> ListItem<'static> {
    let checkbox = match f.state {
        crate::session::ReviewState::Unreviewed => "[ ]",
        crate::session::ReviewState::ReviewedStable => "[x]",
        crate::session::ReviewState::ReviewedDirty => "[~]",
    };
    let (status_char, status_style) = file_status_display(f.status, theme);
    let add_str = format!("+{}", f.additions);
    let del_str = format!("-{}", f.deletions);
    let stats_str = format!(" {checkbox}  {status_char}  {:>count_width$}  {:>count_width$}", add_str, del_str);
    let stats_len = stats_str.chars().count();
    let full_left = format!("{}{}", prefix, filename);
    let full_left_len = full_left.chars().count();
    // Pad short filenames to the longest row so all stats share the same x-position.
    let left_pad = max_left_width.saturating_sub(full_left_len);
    let total = row_width as usize;
    // When the whole line fits: right-align stats at the terminal edge.
    // When stats overflow: they start at max_left_width+1 and clip uniformly across all rows.
    let right_pad = total.saturating_sub(max_left_width + 1 + stats_len);
    let mut spans = vec![
        Span::raw(full_left),
        Span::raw(" ".repeat(left_pad + right_pad + 1)),
        Span::raw(format!("{checkbox} ")),
        Span::styled(status_char, status_style),
        Span::raw("  "),
        Span::styled(format!("{:>count_width$}", add_str), count_style(f.additions, theme.count_positive, theme.count_positive_bg)),
        Span::raw("  "),
        Span::styled(format!("{:>count_width$}", del_str), count_style(f.deletions, theme.count_negative, theme.count_negative_bg)),
    ];
    if let crate::git::FileStatus::Moved = f.status {
        if let Some(old) = &f.old_path {
            spans.push(Span::styled(
                format!("  ({})", old.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()),
                Style::default().add_modifier(Modifier::DIM),
            ));
        }
    }
    ListItem::new(Line::from(spans))
}

fn file_status_display(status: crate::git::FileStatus, theme: &Theme) -> (&'static str, Style) {
    match status {
        crate::git::FileStatus::Added    => ("A", theme.status_added),
        crate::git::FileStatus::Deleted  => ("D", theme.status_deleted),
        crate::git::FileStatus::Modified => ("M", theme.status_modified),
        crate::git::FileStatus::Moved    => ("R", theme.status_moved),
    }
}

fn build_file_paths(files: &[crate::session::FileEntry], worktree_a: &Path, worktree_b: &Path) -> Vec<(PathBuf, PathBuf)> {
    files.iter().map(|f| (worktree_a.join(&f.path), worktree_b.join(&f.path))).collect()
}

fn active_notification(notification: &Option<(String, std::time::Instant)>) -> Option<&str> {
    notification.as_ref().map(|(s, _)| s.as_str())
}

fn diff_line_numbers(text: &ratatui::text::Text, scroll: usize, width: u16) -> (Option<u32>, Option<u32>) {
    let mid = (width / 2) as usize;
    // Scan forward from scroll: header and separator lines carry no file line numbers.
    for line in text.lines.iter().skip(scroll) {
        let chars: Vec<char> = line.spans.iter().flat_map(|s| s.content.chars()).collect();
        let old_line = leading_number_in_chars(&chars[..mid.min(chars.len())]);
        let new_line = leading_number_in_chars(&chars[mid.min(chars.len())..]);
        if old_line.is_some() || new_line.is_some() {
            return (old_line, new_line);
        }
    }
    (None, None)
}

fn leading_number_in_chars(chars: &[char]) -> Option<u32> {
    let start = chars.iter().position(|c| !c.is_whitespace())?;
    let digits: String = chars[start..].iter().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() { None } else { digits.parse().ok() }
}

fn capture_diff(diff_tool: &str, path_a: &Path, path_b: &Path, width: u16, diff_added: Color, diff_deleted: Color, diff_unmatched: Color) -> Result<Text<'static>> {
    let null = Path::new("/dev/null");
    let mut parts = diff_tool.split_whitespace();
    let bin = parts.next().unwrap_or(diff_tool);

    let output = std::process::Command::new(bin)
        .args(parts)
        .arg(if path_a.exists() { path_a } else { null })
        .arg(if path_b.exists() { path_b } else { null })
        .env("CLICOLOR_FORCE", "1")
        .env("COLUMNS", width.to_string())
        .stdout(Stdio::piped())
        .output()?;

    Ok(crate::ansi::parse(&output.stdout, diff_added, diff_deleted, diff_unmatched))
}

fn handle_checklist_key(key: KeyEvent, checklist: &mut Checklist) -> ChecklistControl {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => ChecklistControl::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            ChecklistControl::Quit
        }
        KeyCode::Char('u') | KeyCode::Down => ChecklistControl::SelectAdjacentFile(1),
        KeyCode::Char('i') | KeyCode::Up => ChecklistControl::SelectAdjacentFile(-1),
        KeyCode::Char(' ') => {
            checklist.toggle_reviewed();
            ChecklistControl::Save
        }
        KeyCode::Char('r') => ChecklistControl::Refresh,
        KeyCode::Char('t') => ChecklistControl::CycleTheme,
        KeyCode::Char('v') => ChecklistControl::ToggleView,
        KeyCode::Enter => ChecklistControl::ToggleFullscreen,
        KeyCode::Char('y') => ChecklistControl::CopyPathNew,
        KeyCode::Char('Y') => ChecklistControl::CopyPathOld,
        KeyCode::Char('=') => ChecklistControl::AdjustSplit(-1),
        KeyCode::Char('-') => ChecklistControl::AdjustSplit(1),
        KeyCode::Char('j') => ChecklistControl::ScrollPreview(1),
        KeyCode::Char('k') => ChecklistControl::ScrollPreview(-1),
        _ => ChecklistControl::Continue,
    }
}

fn handle_mouse(
    mouse: MouseEvent,
    split_row: u16,
    dragging: &mut bool,
    fullscreen: bool,
    width: u16,
) -> ChecklistControl {
    // Row 0 = title bar, rows 1..=split_row = file list, row 1+split_row = divider.
    let divider_row = 1 + split_row;
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if mouse.row == divider_row && !fullscreen {
                *dragging = true;
            } else if mouse.row > 0 && mouse.row < divider_row {
                let list_row = mouse.row.saturating_sub(1) as usize;
                return ChecklistControl::SelectVisualRow(list_row);
            } else if mouse.row > divider_row {
                if mouse.column < width / 2 {
                    return ChecklistControl::CopyPathOld;
                } else {
                    return ChecklistControl::CopyPathNew;
                }
            }
            ChecklistControl::Continue
        }
        MouseEventKind::Drag(MouseButton::Left) if *dragging => {
            ChecklistControl::UpdateDrag(mouse.row)
        }
        MouseEventKind::Up(MouseButton::Left) => {
            *dragging = false;
            ChecklistControl::Continue
        }
        MouseEventKind::ScrollDown if mouse.row > divider_row => ChecklistControl::ScrollPreview(3),
        MouseEventKind::ScrollUp   if mouse.row > divider_row => ChecklistControl::ScrollPreview(-3),
        _ => ChecklistControl::Continue,
    }
}
