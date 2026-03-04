use std::path::{Path, PathBuf};

use crate::scheduler::SignalId;

use super::record::{ConfigError, Record};

/// A configuration record backed by a file path.
///
/// `load()` and `save()` are stubs — real I/O is deferred to a later phase.
pub struct ConfigModel<T: Record> {
    value: T,
    path: PathBuf,
    change_signal: SignalId,
    dirty: bool,
}

impl<T: Record> ConfigModel<T> {
    pub fn new(value: T, path: PathBuf, signal_id: SignalId) -> Self {
        Self {
            value,
            path,
            change_signal: signal_id,
            dirty: false,
        }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    /// Replace the value. Returns `true` if dirty flag was set (always, since
    /// Record types don't require PartialEq).
    pub fn set(&mut self, new_value: T) -> bool {
        self.value = new_value;
        self.dirty = true;
        true
    }

    /// Modify the value in place. Returns `true` (marks dirty).
    pub fn modify<F: FnOnce(&mut T)>(&mut self, f: F) -> bool {
        f(&mut self.value);
        self.dirty = true;
        true
    }

    pub fn change_signal(&self) -> SignalId {
        self.change_signal
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Reset the value to its default. Returns `true` if dirty flag was set.
    pub fn reset_to_default(&mut self) -> bool {
        self.value.set_to_default();
        self.dirty = true;
        true
    }

    /// Stub — real I/O deferred.
    pub fn load(&mut self) -> Result<(), ConfigError> {
        Err(ConfigError::ParseError("load not yet implemented".into()))
    }

    /// Stub — real I/O deferred.
    pub fn save(&self) -> Result<(), ConfigError> {
        Err(ConfigError::ParseError("save not yet implemented".into()))
    }
}
