use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::eyre::{bail, Result};

use crate::git;

pub(crate) enum Worktree {
    Checkout { path: PathBuf },
    WorkingTree { path: PathBuf },
}

impl Worktree {
    pub(crate) fn path(&self) -> &Path {
        match self {
            Worktree::Checkout { path } => path,
            Worktree::WorkingTree { path } => path,
        }
    }
}

pub(crate) fn create(repo_root: &Path, git_ref: &str, worktrees_dir: &Path) -> Result<Worktree> {
    if git_ref == "." {
        return Ok(Worktree::WorkingTree { path: repo_root.to_path_buf() });
    }

    let path = worktrees_dir.join(git::sanitise_ref(git_ref));
    if path.exists() {
        return Ok(Worktree::Checkout { path });
    }

    std::fs::create_dir_all(worktrees_dir)?;

    let status = Command::new("git")
        .args(["worktree", "add", "--detach"])
        .arg(&path)
        .arg(git_ref)
        .current_dir(repo_root)
        .status()?;

    if !status.success() {
        bail!("failed to create worktree for '{git_ref}'");
    }

    Ok(Worktree::Checkout { path })
}

pub(crate) fn remove(repo_root: &Path, worktree: Worktree) -> Result<()> {
    let Worktree::Checkout { path } = worktree else {
        return Ok(());
    };

    let status = Command::new("git")
        .args(["worktree", "remove", "--force"])
        .arg(&path)
        .current_dir(repo_root)
        .status()?;

    if !status.success() {
        bail!("failed to remove worktree at '{}'", path.display());
    }

    Ok(())
}

