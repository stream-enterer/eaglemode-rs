use std::path::{Path, PathBuf};

use crate::scheduler::SignalId;

/// Loading state for a file-backed model.
#[derive(Debug, Clone, PartialEq)]
pub enum FileState {
    Waiting,
    Loading { progress: u8 },
    Loaded,
    LoadError(String),
    TooCostly,
}

/// A file-backed data model with a loading state machine.
///
/// `try_continue_loading()` is a stub that immediately transitions to `LoadError`.
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

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn change_signal(&self) -> SignalId {
        self.change_signal
    }

    pub fn set_memory_limit(&mut self, limit: usize) {
        self.memory_limit = limit;
    }

    pub fn progress(&self) -> u8 {
        match &self.state {
            FileState::Loading { progress } => *progress,
            FileState::Loaded => 100,
            _ => 0,
        }
    }

    /// Begin loading. Returns `true` if state changed from `Waiting`.
    pub fn request_load(&mut self) -> bool {
        if self.state != FileState::Waiting {
            return false;
        }
        self.state = FileState::Loading { progress: 0 };
        true
    }

    /// Stub — immediately transitions to `LoadError`. Returns `true` if state changed.
    pub fn try_continue_loading(&mut self) -> bool {
        if !matches!(self.state, FileState::Loading { .. }) {
            return false;
        }
        self.state = FileState::LoadError("loading not yet implemented".into());
        true
    }

    /// Reset to `Waiting` and clear data. Returns `true` if state changed.
    pub fn reset(&mut self) -> bool {
        if self.state == FileState::Waiting && self.data.is_none() {
            return false;
        }
        self.data = None;
        self.state = FileState::Waiting;
        true
    }
}
