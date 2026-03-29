use std::path::{Path, PathBuf};

use color_eyre::eyre::Result;
use git2::{Delta, Oid, Repository};
use serde::{Deserialize, Serialize};

pub(crate) struct Session {
    pub(crate) id: String,
    pub(crate) ref_a: String,
    pub(crate) ref_b: String,
    pub(crate) files: Vec<FileEntry>,
}

pub(crate) struct FileEntry {
    pub(crate) path: PathBuf,
    pub(crate) state: ReviewState,
    // Blob hashes at the time this entry was last computed. None when the
    // side is the working tree (no stable hash) or when the file is
    // absent on that side (added/deleted).
    pub(crate) blob_a: Option<Oid>,
    pub(crate) blob_b: Option<Oid>,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ReviewState {
    Unreviewed,
    ReviewedStable,
}

impl Session {
    pub(crate) fn load_or_create(
        repo: &Repository,
        repo_root: &Path,
        ref_a: &str,
        ref_b: &str,
    ) -> Result<Self> {
        let id = session_id(ref_a, ref_b);
        let current_files = compute_diff_files(repo, ref_a, ref_b)?;
        let persisted = load_persisted(repo_root, &id)?;
        let files = merge_state(current_files, persisted);
        Ok(Session { id, ref_a: ref_a.to_string(), ref_b: ref_b.to_string(), files })
    }

    pub(crate) fn save(&self, repo_root: &Path) -> Result<()> {
        let state_path = session_dir(repo_root, &self.id).join("state.toml");
        std::fs::create_dir_all(state_path.parent().unwrap())?;
        let persisted = self.to_persisted();
        std::fs::write(&state_path, toml::to_string(&persisted)?)?;
        Ok(())
    }

    fn to_persisted(&self) -> PersistedSession {
        PersistedSession {
            files: self.files.iter().map(|f| PersistedFile {
                path: f.path.to_string_lossy().into_owned(),
                state: match f.state {
                    ReviewState::Unreviewed => PersistedState::Unreviewed,
                    ReviewState::ReviewedStable => PersistedState::Reviewed,
                },
                blob_a: f.blob_a.map(|oid| oid.to_string()),
                blob_b: f.blob_b.map(|oid| oid.to_string()),
            }).collect(),
        }
    }
}

// --- Persistence types ---

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedSession {
    files: Vec<PersistedFile>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedFile {
    path: String,
    state: PersistedState,
    #[serde(skip_serializing_if = "Option::is_none")]
    blob_a: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blob_b: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum PersistedState {
    Unreviewed,
    Reviewed,
}

// --- Helpers ---

fn session_id(ref_a: &str, ref_b: &str) -> String {
    format!("{}__{}", sanitise_ref(ref_a), sanitise_ref(ref_b))
}

fn sanitise_ref(git_ref: &str) -> String {
    git_ref.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|', ' '], "-")
}

fn session_dir(repo_root: &Path, id: &str) -> PathBuf {
    repo_root.join(".grit").join(id)
}

fn load_persisted(repo_root: &Path, id: &str) -> Result<Option<PersistedSession>> {
    let path = session_dir(repo_root, id).join("state.toml");
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)?;
    Ok(Some(toml::from_str(&raw)?))
}

fn merge_state(
    current: Vec<(PathBuf, Option<Oid>, Option<Oid>)>,
    persisted: Option<PersistedSession>,
) -> Vec<FileEntry> {
    let persisted = persisted.unwrap_or(PersistedSession { files: vec![] });

    current.into_iter().map(|(path, blob_a, blob_b)| {
        let state = persisted.files.iter()
            .find(|f| Path::new(&f.path) == path)
            .and_then(|f| {
                if f.state == PersistedState::Reviewed
                    && blobs_match(blob_a, &f.blob_a)
                    && blobs_match(blob_b, &f.blob_b)
                {
                    Some(ReviewState::ReviewedStable)
                } else {
                    None
                }
            })
            .unwrap_or(ReviewState::Unreviewed);

        FileEntry { path, state, blob_a, blob_b }
    }).collect()
}

fn blobs_match(current: Option<Oid>, persisted: &Option<String>) -> bool {
    match (current, persisted) {
        (None, None) => true,
        (Some(oid), Some(s)) => oid.to_string() == *s,
        _ => false,
    }
}

fn compute_diff_files(
    repo: &Repository,
    ref_a: &str,
    ref_b: &str,
) -> Result<Vec<(PathBuf, Option<Oid>, Option<Oid>)>> {
    let diff = if ref_b == "." {
        let tree_a = resolve_tree(repo, ref_a)?;
        repo.diff_tree_to_workdir_with_index(Some(&tree_a), None)?
    } else if ref_a == "." {
        // Diff from ref_b tree to working directory; blob sides will be swapped below.
        let tree_b = resolve_tree(repo, ref_b)?;
        repo.diff_tree_to_workdir_with_index(Some(&tree_b), None)?
    } else {
        let tree_a = resolve_tree(repo, ref_a)?;
        let tree_b = resolve_tree(repo, ref_b)?;
        repo.diff_tree_to_tree(Some(&tree_a), Some(&tree_b), None)?
    };

    let mut files = Vec::new();
    for delta in diff.deltas() {
        if matches!(
            delta.status(),
            Delta::Unmodified | Delta::Ignored | Delta::Untracked
        ) {
            continue;
        }

        let path = delta.new_file().path()
            .or_else(|| delta.old_file().path())
            .map(PathBuf::from)
            .unwrap_or_default();

        let tree_blob = non_zero_oid(delta.old_file().id());
        let other_blob = non_zero_oid(delta.new_file().id());

        // When ref_a is ".", the working tree is on the old side; assign None
        // to signal an unstable hash. Otherwise the layout matches the diff
        // direction (old = ref_a, new = ref_b).
        let (blob_a, blob_b) = if ref_a == "." {
            (None, tree_blob)
        } else {
            (tree_blob, if ref_b == "." { None } else { other_blob })
        };

        files.push((path, blob_a, blob_b));
    }

    Ok(files)
}

fn resolve_tree<'repo>(repo: &'repo Repository, git_ref: &str) -> Result<git2::Tree<'repo>> {
    let obj = repo.revparse_single(git_ref)?;
    Ok(obj.peel_to_tree()?)
}

fn non_zero_oid(oid: Oid) -> Option<Oid> {
    if oid.is_zero() { None } else { Some(oid) }
}
