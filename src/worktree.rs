use std::io::Write;
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

pub(crate) fn is_managed(worktree_root: &Path, repo_root: &Path) -> bool {
    worktree_root.starts_with(repo_root.join(".grit").join("worktrees"))
}

fn has_uncommitted_changes(path: &Path) -> Result<bool> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(path)
        .args(["status", "--porcelain"])
        .output()?;

    if !output.status.success() {
        bail!("git failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(!output.stdout.is_empty())
}

fn has_unpushed_commits(path: &Path) -> Result<bool> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(path)
        .args(["log", "HEAD", "--not", "--remotes=*", "--oneline"])
        .output()?;

    if !output.status.success() {
        bail!("git failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(!output.stdout.is_empty())
}

pub(crate) fn remove(repo_root: &Path, worktree: Worktree) -> Result<()> {
    let Worktree::Checkout { path } = worktree else {
        return Ok(());
    };

    if !is_managed(&path, repo_root) && !confirmed_safe_to_remove(&path)? {
        return Ok(());
    }

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

fn confirmed_safe_to_remove(path: &Path) -> Result<bool> {
    let mut warnings: Vec<&str> = Vec::new();

    if has_uncommitted_changes(path)? {
        warnings.push("uncommitted changes");
    }
    if has_unpushed_commits(path)? {
        warnings.push("unpushed commits");
    }

    if warnings.is_empty() {
        return Ok(true);
    }

    eprintln!("Worktree at '{}' has {}.", path.display(), warnings.join(" and "));
    eprint!("Remove anyway? [y/N] ");
    std::io::stderr().flush()?;

    let mut response = String::new();
    std::io::stdin().read_line(&mut response)?;

    Ok(response.trim().eq_ignore_ascii_case("y"))
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::process::Command;

    use super::{has_uncommitted_changes, has_unpushed_commits};

    fn git(dir: &Path, args: &[&str]) {
        let ok = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?} failed");
    }

    fn repo_with_commit(dir: &Path) {
        git(dir, &["init", "-b", "main"]);
        std::fs::write(dir.join("file.txt"), "content").unwrap();
        git(dir, &["add", "."]);
        git(dir, &["commit", "-m", "initial"]);
    }

    #[test]
    fn has_uncommitted_changes_returns_false_for_clean_checkout() {
        let dir = tempfile::tempdir().unwrap();
        repo_with_commit(dir.path());
        assert!(!has_uncommitted_changes(dir.path()).unwrap());
    }

    #[test]
    fn has_uncommitted_changes_returns_true_for_dirty_checkout() {
        let dir = tempfile::tempdir().unwrap();
        repo_with_commit(dir.path());
        std::fs::write(dir.path().join("file.txt"), "modified").unwrap();
        assert!(has_uncommitted_changes(dir.path()).unwrap());
    }

    #[test]
    fn has_unpushed_commits_returns_true_when_no_remote() {
        let dir = tempfile::tempdir().unwrap();
        repo_with_commit(dir.path());
        assert!(has_unpushed_commits(dir.path()).unwrap());
    }

    #[test]
    fn has_unpushed_commits_returns_false_when_synced_with_remote() {
        let work = tempfile::tempdir().unwrap();
        let bare = tempfile::tempdir().unwrap();

        git(bare.path(), &["init", "--bare", "-b", "main"]);
        repo_with_commit(work.path());
        git(work.path(), &["remote", "add", "origin", bare.path().to_str().unwrap()]);
        git(work.path(), &["push", "origin", "main"]);

        assert!(!has_unpushed_commits(work.path()).unwrap());
    }
}
