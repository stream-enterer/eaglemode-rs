use crate::input::{InputEvent, InputKey, InputState, InputVariant};

use super::view::View;

/// Trait for view input filters that intercept input before it reaches panels.
pub trait ViewInputFilter {
    /// Process an input event. Returns true if the event was consumed.
    fn filter(&mut self, event: &InputEvent, state: &InputState, view: &mut View) -> bool;
}

/// Mouse wheel zoom and middle-button pan filter.
pub struct MouseZoomScrollVIF {
    /// Zoom speed multiplier.
    pub zoom_speed: f64,
    /// Whether middle-button panning is active.
    panning: bool,
    last_x: f64,
    last_y: f64,
}

impl MouseZoomScrollVIF {
    pub fn new() -> Self {
        Self {
            zoom_speed: 1.1,
            panning: false,
            last_x: 0.0,
            last_y: 0.0,
        }
    }
}

impl Default for MouseZoomScrollVIF {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewInputFilter for MouseZoomScrollVIF {
    fn filter(&mut self, event: &InputEvent, state: &InputState, view: &mut View) -> bool {
        if event.key == InputKey::MouseMiddle {
            match event.variant {
                InputVariant::Press => {
                    self.panning = true;
                    self.last_x = state.mouse_x;
                    self.last_y = state.mouse_y;
                    return true;
                }
                InputVariant::Release => {
                    self.panning = false;
                    return true;
                }
                _ => {}
            }
        }

        // Handle panning with mouse movement (tracked externally)
        if self.panning {
            let dx = state.mouse_x - self.last_x;
            let dy = state.mouse_y - self.last_y;
            if dx != 0.0 || dy != 0.0 {
                view.scroll(dx, dy);
                self.last_x = state.mouse_x;
                self.last_y = state.mouse_y;
            }
        }

        false
    }
}

/// Keyboard zoom and scroll filter (arrow keys, Page Up/Down).
pub struct KeyboardZoomScrollVIF {
    /// Scroll speed in pixels per key press.
    pub scroll_speed: f64,
    /// Zoom speed multiplier per key press.
    pub zoom_speed: f64,
}

impl KeyboardZoomScrollVIF {
    pub fn new() -> Self {
        Self {
            scroll_speed: 50.0,
            zoom_speed: 1.2,
        }
    }
}

impl Default for KeyboardZoomScrollVIF {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewInputFilter for KeyboardZoomScrollVIF {
    fn filter(&mut self, event: &InputEvent, _state: &InputState, view: &mut View) -> bool {
        if event.variant != InputVariant::Press && event.variant != InputVariant::Repeat {
            return false;
        }

        match event.key {
            InputKey::ArrowUp => {
                view.scroll(0.0, -self.scroll_speed);
                true
            }
            InputKey::ArrowDown => {
                view.scroll(0.0, self.scroll_speed);
                true
            }
            InputKey::ArrowLeft => {
                view.scroll(-self.scroll_speed, 0.0);
                true
            }
            InputKey::ArrowRight => {
                view.scroll(self.scroll_speed, 0.0);
                true
            }
            InputKey::PageUp => {
                let (_, h) = view.viewport_size();
                view.scroll(0.0, -h * 0.8);
                true
            }
            InputKey::PageDown => {
                let (_, h) = view.viewport_size();
                view.scroll(0.0, h * 0.8);
                true
            }
            InputKey::Home => {
                view.go_home();
                true
            }
            _ => false,
        }
    }
}
