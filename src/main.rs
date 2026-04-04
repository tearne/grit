mod ansi;
mod checklist;
mod config;
mod git;
mod open_command;
mod session;
mod theme;
mod tui;
mod worktree;

use clap::Parser;
use color_eyre::eyre::{bail, Result};

#[derive(Parser)]
#[command(name = "grit", about = "Git Review In Terminal", version)]
struct Args {
    ref_a: String,
    ref_b: Option<String>,
}

impl Args {
    fn resolve(self) -> (String, String) {
        (self.ref_a, self.ref_b.unwrap_or_else(|| ".".to_string()))
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    run(args)
}

fn run(args: Args) -> Result<()> {
    let (ref_a, ref_b) = args.resolve();
    validate_refs(&ref_a, &ref_b)?;

    let repo_root = git::repo_root()?;

    let config = config::Config::load()?;
    let theme = theme::Theme::from_name(&config.theme)?;

    let session_key = session::session_id(&ref_a, &ref_b);
    let worktrees_dir = repo_root.join(".grit").join("worktrees").join(session_key);
    let worktree_a = worktree::create(&repo_root, &ref_a, &worktrees_dir)?;
    let worktree_b = worktree::create(&repo_root, &ref_b, &worktrees_dir)?;

    let session =
        session::Session::load_or_create(&repo_root, &ref_a, &ref_b)?;
    let mut checklist = checklist::Checklist::new(session);

    let mut tui = tui::Tui::new()?;
    tui.run(
        &mut checklist,
        &repo_root,
        &config.diff_tool,
        theme,
        worktree_a.path(),
        worktree_b.path(),
        config.auto_refresh,
        config.preview_split,
    )?;

    // Restore terminal before prompting — the TUI holds raw mode until dropped.
    drop(tui);

    worktree::remove(&repo_root, worktree_a)?;
    worktree::remove(&repo_root, worktree_b)?;

    Ok(())
}

fn validate_refs(ref_a: &str, ref_b: &str) -> Result<()> {
    if ref_a == "." && ref_b == "." {
        bail!("'.' may only appear once");
    }
    Ok(())
}
