mod checklist;
mod clipboard;
mod config;
mod session;
mod tui;
mod worktree;

use clap::Parser;
use color_eyre::eyre::{bail, Result, WrapErr};
use git2::Repository;

#[derive(Parser)]
#[command(name = "grit", about = "Git Review In Terminal")]
struct Args {
    ref_a: String,
    ref_b: String,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    run(args)
}

fn run(args: Args) -> Result<()> {
    validate_refs(&args.ref_a, &args.ref_b)?;

    let repo = Repository::discover(".").wrap_err("not inside a git repository")?;
    let repo_root = repo
        .workdir()
        .ok_or_else(|| color_eyre::eyre::eyre!("bare repositories are not supported"))?
        .to_path_buf();

    let config = config::Config::load(&repo_root)?;

    let worktrees_dir = repo_root.join(".grit").join("worktrees");
    let worktree_a = worktree::create(&repo_root, &args.ref_a, &worktrees_dir)?;
    let worktree_b = worktree::create(&repo_root, &args.ref_b, &worktrees_dir)?;

    let session =
        session::Session::load_or_create(&repo, &repo_root, &args.ref_a, &args.ref_b)?;
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
