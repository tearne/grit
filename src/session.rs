use std::path::{Path, PathBuf};

use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};

use crate::git;

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
    pub(crate) blob_a: Option<String>,
    pub(crate) blob_b: Option<String>,
    // Derived from git on each load; not persisted.
    pub(crate) status: git::FileStatus,
    pub(crate) old_path: Option<PathBuf>,
    pub(crate) additions: u32,
    pub(crate) deletions: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ReviewState {
    Unreviewed,
    ReviewedStable,
    ReviewedDirty,
}

impl Session {
    pub(crate) fn load_or_create(
        repo_root: &Path,
        ref_a: &str,
        ref_b: &str,
    ) -> Result<Self> {
        let id = session_id(ref_a, ref_b);
        let current_files = git::diff_files(repo_root, ref_a, ref_b)?;
        let persisted = load_persisted(repo_root, &id)?;
        let files = merge_state(current_files, persisted);
        Ok(Session { id, ref_a: ref_a.to_string(), ref_b: ref_b.to_string(), files })
    }

    pub(crate) fn refresh(&mut self, repo_root: &Path) -> Result<()> {
        let snapshot = self.to_persisted();
        let current_files = git::diff_files(repo_root, &self.ref_a, &self.ref_b)?;
        self.files = merge_state(current_files, Some(snapshot));
        Ok(())
    }

    pub(crate) fn has_pending_changes(&self, repo_root: &Path) -> Result<bool> {
        let snapshot = self.to_persisted();
        let current_files = git::diff_files(repo_root, &self.ref_a, &self.ref_b)?;
        let refreshed = merge_state(current_files, Some(snapshot));
        Ok(differs(&self.files, &refreshed))
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
                    ReviewState::ReviewedStable | ReviewState::ReviewedDirty => PersistedState::Reviewed,
                },
                blob_a: f.blob_a.clone(),
                blob_b: f.blob_b.clone(),
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
struct PersistedFile {
    path: String,
    state: PersistedState,
    #[serde(skip_serializing_if = "Option::is_none")]
    blob_a: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blob_b: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
enum PersistedState {
    Unreviewed,
    Reviewed,
}

// --- Helpers ---

pub(crate) fn session_id(ref_a: &str, ref_b: &str) -> String {
    format!("{}__{}", git::sanitise_ref(ref_a), git::sanitise_ref(ref_b))
}

fn session_dir(repo_root: &Path, id: &str) -> PathBuf {
    repo_root.join(".grit").join("sessions").join(id)
}

fn load_persisted(repo_root: &Path, id: &str) -> Result<Option<PersistedSession>> {
    let path = session_dir(repo_root, id).join("state.toml");
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)?;
    Ok(Some(toml::from_str(&raw)?))
}

#[cfg(test)]
pub(crate) fn differs_for_test(current: &[FileEntry], refreshed: &[FileEntry]) -> bool {
    differs(current, refreshed)
}

fn differs(current: &[FileEntry], refreshed: &[FileEntry]) -> bool {
    if current.len() != refreshed.len() {
        return true;
    }
    current.iter().zip(refreshed.iter()).any(|(a, b)| {
        a.path != b.path
            || a.blob_a != b.blob_a
            || a.blob_b != b.blob_b
            || is_reviewed(a.state) != is_reviewed(b.state)
    })
}

// ReviewedDirty and ReviewedStable are both "reviewed" — the dirty flag
// reflects working-tree state that fluctuates independently of the diff.
// Only the reviewed/unreviewed boundary is a genuine pending change.
fn is_reviewed(state: ReviewState) -> bool {
    matches!(state, ReviewState::ReviewedStable | ReviewState::ReviewedDirty)
}

fn merge_state(
    current: Vec<git::DiffEntry>,
    persisted: Option<PersistedSession>,
) -> Vec<FileEntry> {
    let persisted = persisted.unwrap_or(PersistedSession { files: vec![] });

    current.into_iter().map(|entry| {
        let state = persisted.files.iter()
            .find(|f| Path::new(&f.path) == entry.path)
            .and_then(|f| {
                if f.state != PersistedState::Reviewed {
                    return None;
                }
                if entry.blob_a == f.blob_a && entry.blob_b == f.blob_b {
                    Some(ReviewState::ReviewedStable)
                } else {
                    Some(ReviewState::ReviewedDirty)
                }
            })
            .unwrap_or(ReviewState::Unreviewed);

        FileEntry {
            path: entry.path,
            state,
            blob_a: entry.blob_a,
            blob_b: entry.blob_b,
            status: entry.status,
            old_path: entry.old_path,
            additions: entry.additions,
            deletions: entry.deletions,
        }
    }).collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        load_persisted, merge_state, session_id, PersistedFile, PersistedSession,
        PersistedState, ReviewState, Session,
    };
    use crate::git::{DiffEntry, FileStatus};

    fn oid(hex: &str) -> String {
        format!("{:0<40}", hex)
    }

    fn entry(path: &str, blob_a: Option<String>, blob_b: Option<String>) -> DiffEntry {
        DiffEntry {
            path: PathBuf::from(path),
            blob_a,
            blob_b,
            status: FileStatus::Modified,
            old_path: None,
            additions: 0,
            deletions: 0,
        }
    }

    fn reviewed_persisted(path: &str, blob_a: &str, blob_b: &str) -> PersistedSession {
        PersistedSession {
            files: vec![PersistedFile {
                path: path.into(),
                state: PersistedState::Reviewed,
                blob_a: Some(oid(blob_a)),
                blob_b: Some(oid(blob_b)),
            }],
        }
    }

    // --- session_id ---

    #[test]
    fn session_id_joins_refs() {
        assert_eq!(session_id("main", "feature"), "main__feature");
    }

    #[test]
    fn session_id_sanitises_slashes() {
        assert_eq!(session_id("refs/heads/main", "feature/foo"), "refs-heads-main__feature-foo");
    }

    #[test]
    fn session_id_sanitises_special_chars() {
        assert_eq!(session_id("a:b*c?", "d<e>f"), "a-b-c-__d-e-f");
    }

    // --- merge_state ---

    #[test]
    fn merge_state_no_persisted_all_unreviewed() {
        let entries = merge_state(vec![entry("a.rs", Some(oid("aa")), Some(oid("bb")))], None);
        assert_eq!(entries[0].state, ReviewState::Unreviewed);
    }

    #[test]
    fn merge_state_reviewed_stable_survives_matching_blobs() {
        let persisted = Some(reviewed_persisted("a.rs", "aa", "bb"));
        let entries = merge_state(vec![entry("a.rs", Some(oid("aa")), Some(oid("bb")))], persisted);
        assert_eq!(entries[0].state, ReviewState::ReviewedStable);
    }

    #[test]
    fn merge_state_blob_mismatch_yields_reviewed_dirty() {
        let persisted = Some(reviewed_persisted("a.rs", "aa", "bb"));
        let entries = merge_state(vec![entry("a.rs", Some(oid("aa")), Some(oid("cc")))], persisted);
        assert_eq!(entries[0].state, ReviewState::ReviewedDirty);
    }

    #[test]
    fn merge_state_new_file_is_unreviewed() {
        let entries = merge_state(
            vec![entry("new.rs", Some(oid("aa")), Some(oid("bb")))],
            Some(PersistedSession { files: vec![] }),
        );
        assert_eq!(entries[0].state, ReviewState::Unreviewed);
    }

    #[test]
    fn merge_state_removed_file_is_absent() {
        let persisted = Some(PersistedSession {
            files: vec![PersistedFile {
                path: "gone.rs".into(),
                state: PersistedState::Reviewed,
                blob_a: None,
                blob_b: None,
            }],
        });
        let entries = merge_state(vec![], persisted);
        assert!(entries.is_empty());
    }

    fn working_tree_entry(path: &str, blob_a: &str, blob_b: &str) -> DiffEntry {
        DiffEntry { path: PathBuf::from(path), blob_a: Some(oid(blob_a)), blob_b: Some(oid(blob_b)),
            status: FileStatus::Modified, old_path: None, additions: 0, deletions: 0 }
    }

    fn reviewed_wt_persisted(path: &str, blob_a: &str, blob_b: &str) -> PersistedSession {
        PersistedSession { files: vec![PersistedFile {
            path: path.into(), state: PersistedState::Reviewed,
            blob_a: Some(oid(blob_a)), blob_b: Some(oid(blob_b)),
        }]}
    }

    #[test]
    fn merge_state_yields_reviewed_dirty_when_working_tree_blob_changes() {
        let persisted = Some(reviewed_wt_persisted("a.rs", "aa", "v1"));
        let entries = merge_state(vec![working_tree_entry("a.rs", "aa", "v2")], persisted);
        assert_eq!(entries[0].state, ReviewState::ReviewedDirty);
    }

    #[test]
    fn merge_state_keeps_reviewed_stable_when_working_tree_blob_unchanged() {
        let persisted = Some(reviewed_wt_persisted("a.rs", "aa", "v1"));
        let entries = merge_state(vec![working_tree_entry("a.rs", "aa", "v1")], persisted);
        assert_eq!(entries[0].state, ReviewState::ReviewedStable);
    }

    #[test]
    fn merge_state_reviewed_stable_for_commit_to_commit() {
        let persisted = Some(reviewed_persisted("a.rs", "aa", "bb"));
        let entries = merge_state(vec![entry("a.rs", Some(oid("aa")), Some(oid("bb")))], persisted);
        assert_eq!(entries[0].state, ReviewState::ReviewedStable);
    }

    // --- Session::save + load_persisted round-trip ---

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let repo_root = dir.path();

        let session = Session {
            id: "main__feature".into(),
            ref_a: "main".into(),
            ref_b: "feature".into(),
            files: vec![
                super::FileEntry {
                    path: PathBuf::from("src/lib.rs"),
                    state: ReviewState::ReviewedStable,

                    blob_a: Some(oid("aa")),
                    blob_b: Some(oid("bb")),
                    status: FileStatus::Modified,
                    old_path: None,
                    additions: 5,
                    deletions: 2,
                },
                super::FileEntry {
                    path: PathBuf::from("src/main.rs"),
                    state: ReviewState::Unreviewed,

                    blob_a: Some(oid("cc")),
                    blob_b: Some(oid("dd")),
                    status: FileStatus::Added,
                    old_path: None,
                    additions: 10,
                    deletions: 0,
                },
            ],
        };

        session.save(repo_root).unwrap();

        let loaded = load_persisted(repo_root, "main__feature").unwrap().unwrap();
        assert_eq!(loaded.files.len(), 2);

        assert_eq!(loaded.files[0].path, "src/lib.rs");
        assert_eq!(loaded.files[0].state, PersistedState::Reviewed);
        assert_eq!(loaded.files[0].blob_a, Some(oid("aa")));

        assert_eq!(loaded.files[1].path, "src/main.rs");
        assert_eq!(loaded.files[1].state, PersistedState::Unreviewed);
    }

    // --- has_pending_changes (via differs) ---

    fn unreviewed_entry(path: &str, ba: &str, bb: &str) -> super::FileEntry {
        super::FileEntry {
            path: PathBuf::from(path),
            state: ReviewState::Unreviewed,
            blob_a: Some(oid(ba)),
            blob_b: Some(oid(bb)),
            status: FileStatus::Modified,
            old_path: None,
            additions: 0,
            deletions: 0,
        }
    }

    #[test]
    fn has_pending_changes_false_when_identical() {
        let files = vec![unreviewed_entry("a.rs", "aa", "bb")];
        let refreshed = vec![unreviewed_entry("a.rs", "aa", "bb")];
        assert!(!super::differs_for_test(&files, &refreshed));
    }

    #[test]
    fn has_pending_changes_true_when_blob_changes() {
        let files = vec![unreviewed_entry("a.rs", "aa", "bb")];
        let refreshed = vec![unreviewed_entry("a.rs", "aa", "cc")];
        assert!(super::differs_for_test(&files, &refreshed));
    }

    #[test]
    fn has_pending_changes_true_when_file_added() {
        let files = vec![unreviewed_entry("a.rs", "aa", "bb")];
        let refreshed = vec![
            unreviewed_entry("a.rs", "aa", "bb"),
            unreviewed_entry("b.rs", "cc", "dd"),
        ];
        assert!(super::differs_for_test(&files, &refreshed));
    }

    // --- refresh behaviour (via to_persisted snapshot + merge_state) ---

    fn reviewed_stable_entry(path: &str, ba: &str, bb: &str) -> super::FileEntry {
        super::FileEntry {
            path: PathBuf::from(path),
            state: ReviewState::ReviewedStable,
            blob_a: Some(oid(ba)),
            blob_b: Some(oid(bb)),
            status: FileStatus::Modified,
            old_path: None,
            additions: 0,
            deletions: 0,
        }
    }

    fn snapshot(files: &[super::FileEntry]) -> PersistedSession {
        let s = Session {
            id: "t".into(),
            ref_a: "a".into(),
            ref_b: "b".into(),
            files: files.iter().map(|f| super::FileEntry {
                path: f.path.clone(),
                state: f.state,
                blob_a: f.blob_a.clone(),
                blob_b: f.blob_b.clone(),
                status: FileStatus::Modified,
                old_path: None,
                additions: 0,
                deletions: 0,
            }).collect(),
        };
        s.to_persisted()
    }

    #[test]
    fn refresh_keeps_reviewed_stable_for_unchanged_blobs() {
        let files = [reviewed_stable_entry("a.rs", "aa", "bb")];
        let snap = snapshot(&files);
        let new_diff = vec![entry("a.rs", Some(oid("aa")), Some(oid("bb")))];
        let result = merge_state(new_diff, Some(snap));
        assert_eq!(result[0].state, ReviewState::ReviewedStable);
    }

    #[test]
    fn refresh_yields_reviewed_dirty_for_changed_blobs() {
        let files = [reviewed_stable_entry("a.rs", "aa", "bb")];
        let snap = snapshot(&files);
        let new_diff = vec![entry("a.rs", Some(oid("aa")), Some(oid("cc")))];
        let result = merge_state(new_diff, Some(snap));
        assert_eq!(result[0].state, ReviewState::ReviewedDirty);
    }

    #[test]
    fn refresh_adds_new_files_as_unreviewed() {
        let files = [reviewed_stable_entry("a.rs", "aa", "bb")];
        let snap = snapshot(&files);
        let new_diff = vec![
            entry("a.rs", Some(oid("aa")), Some(oid("bb"))),
            entry("new.rs", Some(oid("cc")), Some(oid("dd"))),
        ];
        let result = merge_state(new_diff, Some(snap));
        assert_eq!(result[1].path, PathBuf::from("new.rs"));
        assert_eq!(result[1].state, ReviewState::Unreviewed);
    }

    #[test]
    fn refresh_removes_files_no_longer_in_diff() {
        let files = [
            reviewed_stable_entry("a.rs", "aa", "bb"),
            reviewed_stable_entry("gone.rs", "cc", "dd"),
        ];
        let snap = snapshot(&files);
        let new_diff = vec![entry("a.rs", Some(oid("aa")), Some(oid("bb")))];
        let result = merge_state(new_diff, Some(snap));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("a.rs"));
    }
}
