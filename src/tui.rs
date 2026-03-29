use std::io;
use std::path::Path;
use std::process::Stdio;
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
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Terminal,
};

use crate::checklist::Checklist;
use crate::clipboard;
use crate::session::ReviewState;

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
        pager: &str,
        worktree_a: &Path,
        worktree_b: &Path,
    ) -> Result<()> {
        loop {
            self.render(checklist)?;
            match self.next_event(checklist)? {
                LoopControl::Continue => {}
                LoopControl::Save => checklist.session.save(repo_root)?,
                LoopControl::OpenDiff => {
                    if let Some(file) = checklist.selected_file() {
                        let path_a = worktree_a.join(&file.path);
                        let path_b = worktree_b.join(&file.path);
                        self.open_diff(diff_tool, pager, &path_a, &path_b)?;
                    }
                }
                LoopControl::CopyPath => {
                    if let Some(file) = checklist.selected_file() {
                        clipboard::copy_open_command(&worktree_b.join(&file.path), 1);
                    }
                }
                LoopControl::Quit => {
                    checklist.session.save(repo_root)?;
                    return Ok(());
                }
            }
        }
    }

    fn open_diff(&mut self, diff_tool: &str, pager: &str, path_a: &Path, path_b: &Path) -> Result<()> {
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

        let null = Path::new("/dev/null");
        let mut diff_parts = diff_tool.split_whitespace();
        let diff_bin = diff_parts.next().unwrap_or(diff_tool);
        let mut diff = std::process::Command::new(diff_bin)
            .args(diff_parts)
            .arg(if path_a.exists() { path_a } else { null })
            .arg(if path_b.exists() { path_b } else { null })
            .stdout(Stdio::piped())
            .spawn()?;

        let mut pager_parts = pager.split_whitespace();
        if let Some(pager_bin) = pager_parts.next() {
            std::process::Command::new(pager_bin)
                .args(pager_parts)
                .stdin(Stdio::from(diff.stdout.take().unwrap()))
                .status()?;
        }

        diff.wait()?;

        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        self.terminal.clear()?;

        Ok(())
    }

    fn render(&mut self, checklist: &Checklist) -> Result<()> {
        let session = &checklist.session;
        let selected = checklist.selected;
        let reviewed = session.files.iter().filter(|f| f.state == ReviewState::ReviewedStable).count();
        let total = session.files.len();

        self.terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
                .split(area);

            let title_bar = Paragraph::new(format!(
                " grit — {}..{}   {}/{} reviewed",
                session.ref_a, session.ref_b, reviewed, total,
            ))
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));
            frame.render_widget(title_bar, chunks[0]);

            let items: Vec<ListItem> = session.files.iter().map(|f| {
                let checkbox = match f.state {
                    ReviewState::Unreviewed => "[ ]",
                    ReviewState::ReviewedStable => "[x]",
                };
                ListItem::new(Line::from(vec![
                    Span::raw(format!(" {checkbox} {}", f.path.display())),
                ]))
            }).collect();

            let mut list_state = ListState::default();
            list_state.select(if total > 0 { Some(selected) } else { None });

            let list = List::new(items).highlight_style(
                Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD),
            );
            frame.render_stateful_widget(list, chunks[1], &mut list_state);

            let footer = Paragraph::new(
                " j/k navigate   Enter open diff   r toggle reviewed   y copy path   q quit",
            )
            .style(Style::default().bg(Color::DarkGray).fg(Color::Gray));
            frame.render_widget(footer, chunks[2]);
        })?;

        Ok(())
    }

    fn next_event(&self, checklist: &mut Checklist) -> Result<LoopControl> {
        if !event::poll(Duration::from_millis(250))? {
            return Ok(LoopControl::Continue);
        }
        match event::read()? {
            Event::Key(key) => Ok(handle_key(key, checklist)),
            Event::Mouse(mouse) => Ok(handle_mouse(mouse, checklist)),
            _ => Ok(LoopControl::Continue),
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

enum LoopControl {
    Continue,
    Save,
    OpenDiff,
    CopyPath,
    Quit,
}

fn handle_key(key: KeyEvent, checklist: &mut Checklist) -> LoopControl {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => LoopControl::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => LoopControl::Quit,
        KeyCode::Char('j') | KeyCode::Down => {
            checklist.select_next();
            LoopControl::Continue
        }
        KeyCode::Char('k') | KeyCode::Up => {
            checklist.select_prev();
            LoopControl::Continue
        }
        KeyCode::Char('r') | KeyCode::Char(' ') => {
            checklist.toggle_reviewed();
            LoopControl::Save
        }
        KeyCode::Enter => LoopControl::OpenDiff,
        KeyCode::Char('y') => LoopControl::CopyPath,
        _ => LoopControl::Continue,
    }
}

fn handle_mouse(mouse: MouseEvent, checklist: &mut Checklist) -> LoopControl {
    // Row 0 is the title bar; file list entries start at row 1.
    if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
        let list_row = mouse.row.saturating_sub(1) as usize;
        checklist.select_index(list_row);
    }
    LoopControl::Continue
}
