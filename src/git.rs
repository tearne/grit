use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::eyre::{bail, eyre, Result};

pub(crate) enum FileStatus {
    Added,
    Deleted,
    Modified,
    Moved,
}

pub(crate) struct DiffEntry {
    pub(crate) path: PathBuf,
    pub(crate) blob_a: Option<String>,
    pub(crate) blob_b: Option<String>,
    pub(crate) status: FileStatus,
    pub(crate) old_path: Option<PathBuf>,
    pub(crate) additions: u32,
    pub(crate) deletions: u32,
}

pub(crate) fn repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;

    if !output.status.success() {
        bail!("not inside a git repository");
    }

    let path = std::str::from_utf8(&output.stdout)
        .map_err(|_| eyre!("git returned non-UTF-8 path"))?
        .trim_end_matches('\n');

    Ok(PathBuf::from(path))
}

/// Blob OIDs are `None` when the side is the working tree (no stable hash)
/// or when the file is absent on that side (added or deleted).
pub(crate) fn diff_files(
    repo_root: &Path,
    ref_a: &str,
    ref_b: &str,
) -> Result<Vec<DiffEntry>> {
    let raw_out = run_git_diff(repo_root, ref_a, ref_b, "--raw")?;
    let numstat_out = run_git_diff(repo_root, ref_a, ref_b, "--numstat")?;

    let raw_entries: Vec<RawEntry> = raw_out.lines().filter_map(parse_raw_line).collect();
    let counts: Vec<(u32, u32)> = numstat_out.lines().filter_map(parse_numstat_line).collect();

    let entries = raw_entries
        .into_iter()
        .enumerate()
        .map(|(i, raw)| {
            // diff-index/diff-tree report from the tree's perspective: old = tree
            // side, new = other side. When the working tree is ref_a, the roles
            // are flipped relative to the session's (ref_a, ref_b) convention.
            let (blob_a, blob_b) = if ref_a == "." {
                (None, raw.blob_old)
            } else {
                (raw.blob_old, if ref_b == "." { None } else { raw.blob_new })
            };
            let (additions, deletions) = counts.get(i).copied().unwrap_or((0, 0));
            DiffEntry { path: raw.path, blob_a, blob_b, status: raw.status, old_path: raw.old_path, additions, deletions }
        })
        .collect();

    Ok(entries)
}

pub(crate) fn dirty_paths(repo_root: &Path) -> Result<HashSet<PathBuf>> {
    let output = Command::new("git")
        .args(["diff-index", "--name-only", "HEAD"])
        .current_dir(repo_root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git failed: {stderr}");
    }

    let stdout = std::str::from_utf8(&output.stdout)
        .map_err(|_| eyre!("git returned non-UTF-8 output"))?;

    Ok(stdout.lines().filter(|l| !l.is_empty()).map(PathBuf::from).collect())
}

pub(crate) fn sanitise_ref(git_ref: &str) -> String {
    if git_ref == "." {
        return "working-tree".to_string();
    }
    git_ref.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|', ' '], "-")
}

fn run_git_diff(repo_root: &Path, ref_a: &str, ref_b: &str, format_flag: &str) -> Result<String> {
    let output = if ref_a == "." || ref_b == "." {
        let tree_ref = if ref_b == "." { ref_a } else { ref_b };
        Command::new("git")
            .args(["diff-index", format_flag, tree_ref])
            .current_dir(repo_root)
            .output()?
    } else {
        Command::new("git")
            .args(["diff-tree", format_flag, "-r", ref_a, ref_b])
            .current_dir(repo_root)
            .output()?
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git failed: {stderr}");
    }

    std::str::from_utf8(&output.stdout)
        .map_err(|_| eyre!("git returned non-UTF-8 output"))
        .map(str::to_string)
}

struct RawEntry {
    path: PathBuf,
    old_path: Option<PathBuf>,
    blob_old: Option<String>,
    blob_new: Option<String>,
    status: FileStatus,
}

// Raw diff line format: :old-mode new-mode old-blob new-blob status\tpath
// Rename/copy:          :old-mode new-mode old-blob new-blob R<n>\told\tnew
// Lines not starting with ':' (e.g. the commit hash from diff-tree) are skipped.
fn parse_raw_line(line: &str) -> Option<RawEntry> {
    let line = line.strip_prefix(':')?;
    let (meta, paths) = line.split_once('\t')?;
    let parts: Vec<&str> = meta.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }

    let blob_old = non_zero_oid(parts[2]);
    let blob_new = non_zero_oid(parts[3]);
    let status_char = parts[4].chars().next()?;

    let (path, old_path, status) = match status_char {
        'A' => (PathBuf::from(paths), None, FileStatus::Added),
        'D' => (PathBuf::from(paths), None, FileStatus::Deleted),
        'R' | 'C' => {
            let (old, new) = paths.split_once('\t')?;
            (PathBuf::from(new), Some(PathBuf::from(old)), FileStatus::Moved)
        }
        _ => (PathBuf::from(paths), None, FileStatus::Modified),
    };

    Some(RawEntry { path, old_path, blob_old, blob_new, status })
}

// Numstat line format: additions\tdeletions\tpath
// Binary files show '-\t-\tpath' — treated as 0/0.
// Lines without three tab-separated fields (e.g. commit hash) are skipped.
fn parse_numstat_line(line: &str) -> Option<(u32, u32)> {
    let mut parts = line.splitn(3, '\t');
    let add_str = parts.next()?;
    let del_str = parts.next()?;
    let _path = parts.next()?; // require three fields to skip non-numstat lines
    Some((add_str.parse().unwrap_or(0), del_str.parse().unwrap_or(0)))
}

fn non_zero_oid(s: &str) -> Option<String> {
    if s.bytes().all(|b| b == b'0') { None } else { Some(s.to_string()) }
}
