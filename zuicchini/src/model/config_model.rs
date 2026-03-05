use std::path::{Path, PathBuf};

use crate::scheduler::SignalId;

use super::record::{ConfigError, Record};

/// A configuration record backed by a file path with KDL serialization.
///
/// Tracks a dirty flag for unsaved changes. `load()` reads from disk,
/// `save()` writes to disk. `load_or_install()` handles first-run by
/// creating a default config file if none exists.
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

    /// Load the configuration from disk. Parses KDL and deserializes.
    pub fn load(&mut self) -> Result<(), ConfigError> {
        let contents = std::fs::read_to_string(&self.path).map_err(|e| {
            ConfigError::ParseError(format!("failed to read {}: {e}", self.path.display()))
        })?;
        let doc: kdl::KdlDocument = contents
            .parse()
            .map_err(|e| ConfigError::ParseError(format!("KDL parse error: {e}")))?;

        // Look for the first node in the document
        let node = doc
            .nodes()
            .first()
            .ok_or_else(|| ConfigError::ParseError("empty KDL document".into()))?;

        self.value = T::from_kdl(node)?;
        self.dirty = false;
        Ok(())
    }

    /// Save the configuration to disk as KDL.
    pub fn save(&mut self) -> Result<(), ConfigError> {
        let node = self.value.to_kdl();
        let mut doc = kdl::KdlDocument::new();
        doc.nodes_mut().push(node);
        let contents = doc.to_string();

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ConfigError::ParseError(format!("failed to create dir: {e}")))?;
        }

        std::fs::write(&self.path, contents).map_err(|e| {
            ConfigError::ParseError(format!("failed to write {}: {e}", self.path.display()))
        })?;

        self.dirty = false;
        Ok(())
    }

    /// Load from disk, or create a default config file if none exists.
    pub fn load_or_install(&mut self) -> Result<(), ConfigError> {
        if self.path.exists() {
            self.load()
        } else {
            // Create default config and save it
            self.value.set_to_default();
            self.dirty = true;
            self.save()
        }
    }
}
