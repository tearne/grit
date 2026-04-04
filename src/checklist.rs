use std::path::Path;

use color_eyre::eyre::Result;

use crate::session::{FileEntry, ReviewState, Session};

pub(crate) struct Checklist {
    pub(crate) session: Session,
    pub(crate) selected: usize,
}

impl Checklist {
    pub(crate) fn new(session: Session) -> Self {
        Checklist { session, selected: 0 }
    }

    pub(crate) fn selected_file(&self) -> Option<&FileEntry> {
        self.session.files.get(self.selected)
    }

    pub(crate) fn select_next(&mut self) {
        if !self.session.files.is_empty() {
            self.selected = (self.selected + 1).min(self.session.files.len() - 1);
        }
    }

    pub(crate) fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub(crate) fn select_index(&mut self, index: usize) {
        if index < self.session.files.len() {
            self.selected = index;
        }
    }

    pub(crate) fn refresh(&mut self, repo_root: &Path) -> Result<()> {
        self.session.refresh(repo_root)?;
        if !self.session.files.is_empty() {
            self.selected = self.selected.min(self.session.files.len() - 1);
        }
        Ok(())
    }

    pub(crate) fn toggle_reviewed(&mut self) {
        if let Some(entry) = self.session.files.get_mut(self.selected) {
            entry.state = match entry.state {
                ReviewState::Unreviewed    => ReviewState::ReviewedStable,
                ReviewState::ReviewedDirty => ReviewState::ReviewedStable,
                ReviewState::ReviewedStable => ReviewState::Unreviewed,
            };
        }
    }
}
