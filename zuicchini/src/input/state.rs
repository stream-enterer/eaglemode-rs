use std::collections::HashSet;

use super::event::InputKey;

/// Tracks the current state of all input devices.
#[derive(Clone, Debug)]
pub struct InputState {
    /// Currently pressed keys.
    pressed: HashSet<InputKey>,
    /// Current mouse position in window coordinates.
    pub mouse_x: f64,
    pub mouse_y: f64,
    /// Current touch points (id, x, y).
    touches: Vec<(u64, f64, f64)>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            pressed: HashSet::new(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            touches: Vec::new(),
        }
    }

    /// Record a key press.
    pub fn press(&mut self, key: InputKey) {
        self.pressed.insert(key);
    }

    /// Record a key release.
    pub fn release(&mut self, key: InputKey) {
        self.pressed.remove(&key);
    }

    /// Check if a key is currently pressed.
    pub fn is_pressed(&self, key: InputKey) -> bool {
        self.pressed.contains(&key)
    }

    /// Check if Shift is held.
    pub fn shift(&self) -> bool {
        self.is_pressed(InputKey::Shift)
    }

    /// Check if Ctrl is held.
    pub fn ctrl(&self) -> bool {
        self.is_pressed(InputKey::Ctrl)
    }

    /// Check if Alt is held.
    pub fn alt(&self) -> bool {
        self.is_pressed(InputKey::Alt)
    }

    /// Check if Meta (Cmd/Win) is held.
    pub fn meta(&self) -> bool {
        self.is_pressed(InputKey::Meta)
    }

    /// Update mouse position.
    pub fn set_mouse(&mut self, x: f64, y: f64) {
        self.mouse_x = x;
        self.mouse_y = y;
    }

    /// Update a touch point. Inserts if new, updates if existing.
    pub fn set_touch(&mut self, id: u64, x: f64, y: f64) {
        if let Some(touch) = self.touches.iter_mut().find(|t| t.0 == id) {
            touch.1 = x;
            touch.2 = y;
        } else {
            self.touches.push((id, x, y));
        }
    }

    /// Remove a touch point.
    pub fn remove_touch(&mut self, id: u64) {
        self.touches.retain(|t| t.0 != id);
    }

    /// Get all active touch points.
    pub fn touches(&self) -> &[(u64, f64, f64)] {
        &self.touches
    }

    /// Get the set of currently pressed keys.
    pub fn pressed_keys(&self) -> &HashSet<InputKey> {
        &self.pressed
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}
