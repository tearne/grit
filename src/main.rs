mod ansi;
mod checklist;
mod clipboard;
mod config;
mod git;
mod session;
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

    let config = config::Config::load(&repo_root)?;

    let worktrees_dir = repo_root.join(".grit").join("worktrees");
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
        worktree_a.path(),
        worktree_b.path(),
    )?;

    // Reached only on clean exit — unclean exits leave worktrees for reuse.
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
