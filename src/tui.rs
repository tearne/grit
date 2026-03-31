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
    widgets::{List, ListItem, ListState, Paragraph},
    Terminal,
};

use crate::checklist::Checklist;
use crate::clipboard;
use crate::git::FileStatus;
use crate::session::ReviewState;

const BRAILLE_FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

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
        worktree_a: &Path,
        worktree_b: &Path,
    ) -> Result<()> {
        let mut available_height = self.terminal.size()?.height.saturating_sub(3);
        let mut split_row = default_split(available_height);
        let mut dragging = false;
        let mut list_state = ListState::default();
        let mut cached: HashMap<usize, Text<'static>> = HashMap::new();
        let mut spinner_frame: u8 = 0;
        let mut last_selected: Option<usize> = None;

        let file_paths: Vec<(PathBuf, PathBuf)> = checklist
            .session
            .files
            .iter()
            .map(|f| (worktree_a.join(&f.path), worktree_b.join(&f.path)))
            .collect();

        let mut width = self.terminal.size()?.width.saturating_sub(1);
        let worker = DiffWorker::start(work_items(&file_paths, diff_tool, width));

        loop {
            let total = checklist.session.files.len();
            list_state.select(if total > 0 { Some(checklist.selected) } else { None });

            while let Ok((index, text)) = worker.result_rx.try_recv() {
                cached.insert(index, text);
            }

            if total > 0 && last_selected != Some(checklist.selected) {
                last_selected = Some(checklist.selected);
                if !cached.contains_key(&checklist.selected) {
                    worker.prioritize(checklist.selected);
                }
            }

            let all_done = total == 0 || cached.len() == total;
            if !all_done {
                spinner_frame = (spinner_frame + 1) % BRAILLE_FRAMES.len() as u8;
            }

            let preview = cached.get(&checklist.selected);
            self.render_checklist(checklist, &mut list_state, preview, spinner_frame, split_row)?;

            match self.next_checklist_event(checklist, split_row, &mut dragging, !all_done)? {
                ChecklistControl::Continue => {}
                ChecklistControl::Save => checklist.session.save(repo_root)?,
                ChecklistControl::OpenDiff => {
                    if let Some(file) = checklist.selected_file() {
                        let path_a = worktree_a.join(&file.path);
                        let path_b = worktree_b.join(&file.path);
                        let label = file.path.display().to_string();
                        let initial = cached.get(&checklist.selected).cloned();
                        self.run_diff_view(diff_tool, &path_a, &path_b, &label, initial)?;
                    }
                }
                ChecklistControl::CopyPathNew => {
                    if let Some(file) = checklist.selected_file() {
                        clipboard::copy_open_command(&worktree_b.join(&file.path), 1);
                    }
                }
                ChecklistControl::CopyPathOld => {
                    if let Some(file) = checklist.selected_file() {
                        clipboard::copy_open_command(&worktree_a.join(&file.path), 1);
                    }
                }
                ChecklistControl::Quit => {
                    checklist.session.save(repo_root)?;
                    return Ok(());
                }
                ChecklistControl::AdjustSplit(delta) => {
                    let max = available_height.saturating_sub(2);
                    split_row = (split_row as i32 + delta).clamp(2, max as i32) as u16;
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
                    worker.reset(work_items(&file_paths, diff_tool, width));
                }
            }
        }
    }

    fn run_diff_view(
        &mut self,
        diff_tool: &str,
        path_a: &Path,
        path_b: &Path,
        label: &str,
        initial: Option<Text<'static>>,
    ) -> Result<()> {
        let width = self.terminal.size()?.width;
        let mut text = match initial {
            Some(t) => t,
            None => capture_diff(diff_tool, path_a, path_b, width)?,
        };
        let mut scroll: usize = 0;

        loop {
            self.render_diff(&text, scroll, label)?;
            match self.next_diff_event(&text, &mut scroll)? {
                DiffControl::Continue => {}
                DiffControl::Resize => {
                    let new_width = self.terminal.size()?.width;
                    text = capture_diff(diff_tool, path_a, path_b, new_width)?;
                    scroll = 0;
                }
                DiffControl::Exit => return Ok(()),
            }
        }
    }

    fn render_checklist(
        &mut self,
        checklist: &Checklist,
        list_state: &mut ListState,
        preview: Option<&Text<'static>>,
        spinner_frame: u8,
        split_row: u16,
    ) -> Result<()> {
        let session = &checklist.session;
        let total = session.files.len();
        let preview = preview.cloned();

        self.terminal.draw(|frame| {
            let area = frame.area();
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

            let title_bar = Paragraph::new(title_bar_text(&session.ref_a, &session.ref_b, vertical[0].width))
                .style(Style::default().bg(Color::DarkGray).fg(Color::White));
            frame.render_widget(title_bar, vertical[0]);

            // File list with scroll-indicator column.
            let list_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(vertical[1]);

            let max_add = session.files.iter().map(|f| f.additions).max().unwrap_or(0);
            let max_del = session.files.iter().map(|f| f.deletions).max().unwrap_or(0);
            let count_width = format!("+{max_add}").len().max(format!("-{max_del}").len());

            let items: Vec<ListItem> = session
                .files
                .iter()
                .map(|f| {
                    let checkbox = match f.state {
                        ReviewState::Unreviewed => "[ ]",
                        ReviewState::ReviewedStable => "[x]",
                    };
                    let (status_char, status_style) = match f.status {
                        FileStatus::Added    => ("A", Style::default().fg(Color::Green)),
                        FileStatus::Deleted  => ("D", Style::default().fg(Color::Red)),
                        FileStatus::Modified => ("M", Style::default().fg(Color::Yellow)),
                        FileStatus::Moved    => ("R", Style::default().fg(Color::Cyan)),
                    };
                    let add_str = format!("+{}", f.additions);
                    let del_str = format!("-{}", f.deletions);
                    let mut spans = vec![
                        Span::raw(format!(" {checkbox} ")),
                        Span::styled(status_char, status_style),
                        Span::raw("  "),
                        Span::styled(format!("{:>count_width$}", add_str), count_style(f.additions, true)),
                        Span::raw("  "),
                        Span::styled(format!("{:>count_width$}", del_str), count_style(f.deletions, false)),
                        Span::raw(format!("  {}", f.path.display())),
                    ];
                    if let FileStatus::Moved = f.status {
                        if let Some(old) = &f.old_path {
                            spans.push(Span::styled(
                                format!("  (was: {})", old.display()),
                                Style::default().add_modifier(Modifier::DIM),
                            ));
                        }
                    }
                    ListItem::new(Line::from(spans))
                })
                .collect();

            let list = List::new(items).highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_stateful_widget(list, list_chunks[0], list_state);

            let list_height = list_chunks[0].height as usize;
            let offset = list_state.offset();
            let list_indicator =
                scroll_indicator(offset > 0, offset + list_height < total, list_chunks[1].height);
            frame.render_widget(Paragraph::new(list_indicator), list_chunks[1]);

            // Divider — left label for preview, right label for resize hint.
            let left = "─── Preview · Enter to expand ";
            let right = " =/- resize ───";
            let fill_len = (area.width as usize)
                .saturating_sub(left.chars().count() + right.chars().count());
            let divider = Paragraph::new(format!("{left}{}{right}", "─".repeat(fill_len)))
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(divider, vertical[2]);

            // Preview with scroll-indicator column.
            let preview_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(vertical[3]);

            if let Some(text) = &preview {
                frame.render_widget(Paragraph::new(text.clone()), preview_chunks[0]);
                let preview_height = preview_chunks[0].height as usize;
                let has_more = text.lines.len() > preview_height;
                let preview_indicator =
                    scroll_indicator(false, has_more, preview_chunks[1].height);
                frame.render_widget(Paragraph::new(preview_indicator), preview_chunks[1]);
            } else if total > 0 {
                let spinner = BRAILLE_FRAMES[spinner_frame as usize];
                frame.render_widget(
                    Paragraph::new(format!("  {spinner}  analysing…")),
                    preview_chunks[0],
                );
            }

            let footer = Paragraph::new(
                " j/k navigate   r toggle reviewed   y/Y copy path (new/old)",
            )
            .style(Style::default().bg(Color::DarkGray).fg(Color::Gray));
            frame.render_widget(footer, vertical[4]);
        })?;

        Ok(())
    }

    fn render_diff(&mut self, text: &Text<'static>, scroll: usize, label: &str) -> Result<()> {
        self.terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
                .split(area);

            let title_bar = Paragraph::new(format!(" {label}"))
                .style(Style::default().bg(Color::DarkGray).fg(Color::White));
            frame.render_widget(title_bar, chunks[0]);

            let diff_view = Paragraph::new(text.clone()).scroll((scroll as u16, 0));
            frame.render_widget(diff_view, chunks[1]);

            let footer = Paragraph::new(" j/k scroll   Enter/Esc/q return")
                .style(Style::default().bg(Color::DarkGray).fg(Color::Gray));
            frame.render_widget(footer, chunks[2]);
        })?;

        Ok(())
    }

    fn next_checklist_event(
        &self,
        checklist: &mut Checklist,
        split_row: u16,
        dragging: &mut bool,
        loading: bool,
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
            Event::Mouse(mouse) => Ok(handle_mouse(mouse, checklist, split_row, dragging)),
            Event::Resize(_, _) => Ok(ChecklistControl::Resize),
            _ => Ok(ChecklistControl::Continue),
        }
    }

    fn next_diff_event(&self, text: &Text<'static>, scroll: &mut usize) -> Result<DiffControl> {
        if !event::poll(Duration::from_millis(250))? {
            return Ok(DiffControl::Continue);
        }
        match event::read()? {
            Event::Key(key) => Ok(handle_diff_key(key, text, scroll)),
            Event::Resize(_, _) => Ok(DiffControl::Resize),
            _ => Ok(DiffControl::Continue),
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
                        let text = capture_diff(&item.diff_tool, &item.path_a, &item.path_b, item.width)
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

fn work_items(file_paths: &[(PathBuf, PathBuf)], diff_tool: &str, width: u16) -> VecDeque<WorkItem> {
    file_paths
        .iter()
        .enumerate()
        .map(|(index, (path_a, path_b))| WorkItem {
            index,
            path_a: path_a.clone(),
            path_b: path_b.clone(),
            diff_tool: diff_tool.to_string(),
            width,
        })
        .collect()
}

enum ChecklistControl {
    Continue,
    Save,
    OpenDiff,
    CopyPathNew,
    CopyPathOld,
    Quit,
    AdjustSplit(i32),
    UpdateDrag(u16),
    Resize,
}

enum DiffControl {
    Continue,
    Resize,
    Exit,
}

fn title_bar_text(ref_a: &str, ref_b: &str, width: u16) -> String {
    let left = format!("  {}  →  {}", ref_a, ref_b);
    let right = format!("grit v{}  ", env!("CARGO_PKG_VERSION"));
    let padding = (width as usize).saturating_sub(left.len() + right.len());
    format!("{}{}{}", left, " ".repeat(padding), right)
}

fn default_split(available_height: u16) -> u16 {
    (available_height * 2 / 3).max(2)
}

fn scroll_indicator(above: bool, below: bool, height: u16) -> Text<'static> {
    if height == 0 {
        return Text::default();
    }
    let mut lines: Vec<Line<'static>> = (0..height).map(|_| Line::from(" ")).collect();
    if above && below && height == 1 {
        lines[0] = Line::from("↕");
    } else {
        if above {
            lines[0] = Line::from("↑");
        }
        if below {
            lines[height as usize - 1] = Line::from("↓");
        }
    }
    Text::from(lines)
}

fn count_style(count: u32, positive: bool) -> Style {
    let fg = if positive { Color::Green } else { Color::Red };
    let bg = if positive { Color::Rgb(0, 80, 0) } else { Color::Rgb(80, 0, 0) };
    match count {
        0         => Style::default().fg(fg).add_modifier(Modifier::DIM),
        1..=99    => Style::default().fg(fg),
        100..=999 => Style::default().fg(fg).add_modifier(Modifier::BOLD),
        _         => Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    }
}

fn capture_diff(diff_tool: &str, path_a: &Path, path_b: &Path, width: u16) -> Result<Text<'static>> {
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

    Ok(crate::ansi::parse(&output.stdout))
}

fn handle_checklist_key(key: KeyEvent, checklist: &mut Checklist) -> ChecklistControl {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => ChecklistControl::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            ChecklistControl::Quit
        }
        KeyCode::Char('j') | KeyCode::Down => {
            checklist.select_next();
            ChecklistControl::Continue
        }
        KeyCode::Char('k') | KeyCode::Up => {
            checklist.select_prev();
            ChecklistControl::Continue
        }
        KeyCode::Char('r') | KeyCode::Char(' ') => {
            checklist.toggle_reviewed();
            ChecklistControl::Save
        }
        KeyCode::Enter => ChecklistControl::OpenDiff,
        KeyCode::Char('y') => ChecklistControl::CopyPathNew,
        KeyCode::Char('Y') => ChecklistControl::CopyPathOld,
        KeyCode::Char('=') => ChecklistControl::AdjustSplit(1),
        KeyCode::Char('-') => ChecklistControl::AdjustSplit(-1),
        _ => ChecklistControl::Continue,
    }
}

fn handle_diff_key(key: KeyEvent, text: &Text<'static>, scroll: &mut usize) -> DiffControl {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => DiffControl::Exit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => DiffControl::Exit,
        KeyCode::Char('j') | KeyCode::Down => {
            let max = text.lines.len().saturating_sub(1);
            *scroll = (*scroll + 1).min(max);
            DiffControl::Continue
        }
        KeyCode::Char('k') | KeyCode::Up => {
            *scroll = scroll.saturating_sub(1);
            DiffControl::Continue
        }
        _ => DiffControl::Continue,
    }
}

fn handle_mouse(
    mouse: MouseEvent,
    checklist: &mut Checklist,
    split_row: u16,
    dragging: &mut bool,
) -> ChecklistControl {
    // Row 0 = title bar, rows 1..=split_row = file list, row 1+split_row = divider.
    let divider_row = 1 + split_row;
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if mouse.row == divider_row {
                *dragging = true;
            } else if mouse.row > 0 && mouse.row < divider_row {
                let list_row = mouse.row.saturating_sub(1) as usize;
                checklist.select_index(list_row);
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
        _ => ChecklistControl::Continue,
    }
}
