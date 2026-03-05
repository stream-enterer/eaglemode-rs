use std::path::{Path, PathBuf};

use crate::scheduler::SignalId;

/// Loading/saving state for a file-backed model.
///
/// Matches the C++ emFileModel state machine with all 8 states.
#[derive(Debug, Clone, PartialEq)]
pub enum FileState {
    /// Waiting to be loaded (initial state).
    Waiting,
    /// Currently loading with progress (0.0 to 100.0).
    Loading { progress: f64 },
    /// Successfully loaded.
    Loaded,
    /// Data has been modified and not yet saved.
    Unsaved,
    /// Currently saving.
    Saving,
    /// Load failed with an error message.
    LoadError(String),
    /// Save failed with an error message.
    SaveError(String),
    /// Loading would exceed the memory limit.
    TooCostly,
}

/// A file-backed data model with a loading state machine.
///
/// The loading/saving lifecycle is driven by the caller (typically a scheduler
/// engine). The abstract loading/saving operations are implemented via the
/// `FileModelLoader` trait.
pub struct FileModel<T> {
    data: Option<T>,
    path: PathBuf,
    state: FileState,
    change_signal: SignalId,
    memory_limit: usize,
}

impl<T> FileModel<T> {
    pub fn new(path: PathBuf, signal_id: SignalId) -> Self {
        Self {
            data: None,
            path,
            state: FileState::Waiting,
            change_signal: signal_id,
            memory_limit: usize::MAX,
        }
    }

    pub fn state(&self) -> &FileState {
        &self.state
    }

    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    pub fn data_mut(&mut self) -> Option<&mut T> {
        self.data.as_mut()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn change_signal(&self) -> SignalId {
        self.change_signal
    }

    pub fn set_memory_limit(&mut self, limit: usize) {
        self.memory_limit = limit;
    }

    pub fn memory_limit(&self) -> usize {
        self.memory_limit
    }

    pub fn progress(&self) -> f64 {
        match &self.state {
            FileState::Loading { progress } => *progress,
            FileState::Loaded | FileState::Unsaved => 100.0,
            _ => 0.0,
        }
    }

    /// Begin loading. Transitions from `Waiting` to `Loading`.
    /// Also allows retry from `LoadError` and `TooCostly`.
    pub fn request_load(&mut self) -> bool {
        match &self.state {
            FileState::Waiting | FileState::LoadError(_) | FileState::TooCostly => {
                self.state = FileState::Loading { progress: 0.0 };
                true
            }
            _ => false,
        }
    }

    /// Set loading progress (0.0 to 100.0).
    pub fn set_progress(&mut self, progress: f64) {
        if matches!(self.state, FileState::Loading { .. }) {
            self.state = FileState::Loading { progress };
        }
    }

    /// Complete loading with the loaded data.
    pub fn complete_load(&mut self, data: T) {
        self.data = Some(data);
        self.state = FileState::Loaded;
    }

    /// Fail loading with an error message.
    pub fn fail_load(&mut self, error: String) {
        self.state = FileState::LoadError(error);
    }

    /// Mark the data as too costly to load.
    pub fn mark_too_costly(&mut self) {
        self.state = FileState::TooCostly;
    }

    /// Mark data as modified (unsaved).
    pub fn mark_unsaved(&mut self) {
        if matches!(self.state, FileState::Loaded) {
            self.state = FileState::Unsaved;
        }
    }

    /// Begin saving. Transitions from `Unsaved` to `Saving`.
    pub fn request_save(&mut self) -> bool {
        match &self.state {
            FileState::Unsaved | FileState::SaveError(_) => {
                self.state = FileState::Saving;
                true
            }
            _ => false,
        }
    }

    /// Complete saving.
    pub fn complete_save(&mut self) {
        if matches!(self.state, FileState::Saving) {
            self.state = FileState::Loaded;
        }
    }

    /// Fail saving with an error message.
    pub fn fail_save(&mut self, error: String) {
        self.state = FileState::SaveError(error);
    }

    /// Reset to `Waiting` and clear data.
    pub fn reset(&mut self) -> bool {
        if self.state == FileState::Waiting && self.data.is_none() {
            return false;
        }
        self.data = None;
        self.state = FileState::Waiting;
        true
    }
}
