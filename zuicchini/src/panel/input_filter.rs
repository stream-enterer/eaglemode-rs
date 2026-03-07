use crate::input::{InputEvent, InputKey, InputState, InputVariant};

use super::view::{View, ViewFlags};

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
    /// Whether Alt-click middle-button emulation is enabled.
    emulate_middle_button: bool,
    /// Timestamp (ms) of the last emulated middle-button press.
    emu_mid_button_time: u64,
    /// Repeat counter for emulated middle-button double/triple click.
    emu_mid_button_repeat: u32,
    /// Current wheel zoom speed (accumulated with acceleration).
    wheel_zoom_speed: f64,
    /// Timestamp (ms) of the last wheel zoom event.
    wheel_zoom_time: u64,
    /// Spring constant for the mouse swiping animator.
    mouse_spring_const: f64,
    /// Friction for the mouse swiping animator.
    mouse_friction: f64,
    /// Whether kinetic mouse behavior is enabled.
    mouse_friction_enabled: bool,
    /// Spring constant for the wheel swiping animator.
    wheel_spring_const: f64,
    /// Friction for the wheel swiping animator.
    wheel_friction: f64,
    /// Whether kinetic wheel behavior is enabled.
    wheel_friction_enabled: bool,
}

impl MouseZoomScrollVIF {
    pub fn new() -> Self {
        Self {
            zoom_speed: 1.1,
            panning: false,
            last_x: 0.0,
            last_y: 0.0,
            emulate_middle_button: false,
            emu_mid_button_time: 0,
            emu_mid_button_repeat: 0,
            wheel_zoom_speed: 0.0,
            wheel_zoom_time: 0,
            mouse_spring_const: 0.0,
            mouse_friction: 0.0,
            mouse_friction_enabled: false,
            wheel_spring_const: 0.0,
            wheel_friction: 0.0,
            wheel_friction_enabled: false,
        }
    }

    /// Enable or disable Alt-click middle-button emulation.
    pub fn set_emulate_middle_button(&mut self, enabled: bool) {
        self.emulate_middle_button = enabled;
    }

    /// Returns whether middle-button emulation is enabled.
    pub fn emulate_middle_button(&self) -> bool {
        self.emulate_middle_button
    }

    /// Translate Alt key presses into emulated middle mouse button events.
    ///
    /// Mirrors C++ `emMouseZoomScrollVIF::EmulateMiddleButton`.
    /// When emulation is enabled and the real middle button is not pressed,
    /// an Alt key press generates a synthetic middle-button event. Tracks
    /// timing for double/triple click emulation (330ms threshold).
    ///
    /// Returns `Some(synthetic_event)` if an emulated middle-button press
    /// should be generated, or `None` if no emulation occurred. The caller
    /// should process the returned event before normal input handling.
    pub fn emulate_middle_button_event(
        &mut self,
        event: &InputEvent,
        state: &InputState,
        clock_ms: u64,
    ) -> Option<InputEvent> {
        if !self.emulate_middle_button {
            return None;
        }
        // Don't emulate if the real middle button is already held
        if state.is_pressed(InputKey::MouseMiddle) {
            return None;
        }

        if event.key == InputKey::Alt && event.variant == InputVariant::Press && !event.is_repeat {
            // Compute repeat from timing
            let d = clock_ms.saturating_sub(self.emu_mid_button_time);
            if d < 330 {
                self.emu_mid_button_repeat += 1;
            } else {
                self.emu_mid_button_repeat = 0;
            }
            self.emu_mid_button_time = clock_ms;

            // Synthesize a middle button press event
            let mut synthetic = InputEvent::press(InputKey::MouseMiddle);
            synthetic.is_repeat = self.emu_mid_button_repeat > 0;
            synthetic.mouse_x = event.mouse_x;
            synthetic.mouse_y = event.mouse_y;
            return Some(synthetic);
        }

        None
    }

    /// Calculate a new wheel zoom speed with acceleration curve.
    ///
    /// Mirrors C++ `emMouseZoomScrollVIF::UpdateWheelZoomSpeed`.
    /// `down` is true for zoom-out (wheel down), false for zoom-in.
    /// `fine` is true for shift-held fine-mode (0.1x speed).
    /// `clock_ms` is the current timestamp in milliseconds.
    /// `acceleration` is the configured acceleration value (0 = none).
    /// `min_acceleration` is the minimum config value.
    pub fn update_wheel_zoom_speed(
        &mut self,
        down: bool,
        fine: bool,
        clock_ms: u64,
        acceleration: f64,
        min_acceleration: f64,
    ) {
        let mut new_speed = 2.0_f64.sqrt().ln();
        if fine {
            new_speed *= 0.1;
        }
        if down {
            new_speed = -new_speed;
        }

        // Apply acceleration curve if enabled
        if acceleration > min_acceleration * 1.0001 {
            let t1: f64 = 0.03;
            let t2: f64 = 0.35;
            let f1: f64 = 2.2;
            let f2: f64 = 0.4;

            let mut dt = (clock_ms.saturating_sub(self.wheel_zoom_time)) as f64 * 0.001;

            // Direction reversal resets timing
            if new_speed * self.wheel_zoom_speed < 0.0 {
                dt = t2;
            }
            dt = dt.clamp(t1, t2);

            // Exponential interpolation between f1 (fast) and f2 (slow)
            let t_frac = (dt - t1) / (t2 - t1);
            let factor = f1 * (f2 / f1).powf(t_frac);
            new_speed *= factor;
        }

        self.wheel_zoom_speed = new_speed;
        self.wheel_zoom_time = clock_ms;
    }

    /// Returns the current wheel zoom speed.
    pub fn wheel_zoom_speed(&self) -> f64 {
        self.wheel_zoom_speed
    }

    /// Configure mouse swiping animator parameters from kinetic config.
    ///
    /// Mirrors C++ `emMouseZoomScrollVIF::SetMouseAnimParams`.
    /// `kinetic_factor` is the KineticZoomingAndScrolling config value.
    /// `min_kinetic` is the minimum value of that config range.
    /// `zoom_factor_log_per_pixel` is from `View::get_zoom_factor_log_per_pixel`.
    pub fn set_mouse_anim_params(
        &mut self,
        kinetic_factor: f64,
        min_kinetic: f64,
        zoom_factor_log_per_pixel: f64,
    ) {
        let mut k = kinetic_factor;
        if k < min_kinetic * 1.0001 {
            k = 0.001;
        }
        let zflpp = zoom_factor_log_per_pixel.max(1e-10);
        self.mouse_spring_const = 2500.0 / (k * k);
        self.mouse_friction = 2.0 / zflpp / (k * k);
        self.mouse_friction_enabled = k > 0.001;
    }

    /// Returns the mouse animator parameters (spring_const, friction, friction_enabled).
    pub fn mouse_anim_params(&self) -> (f64, f64, bool) {
        (
            self.mouse_spring_const,
            self.mouse_friction,
            self.mouse_friction_enabled,
        )
    }

    /// Configure wheel swiping animator parameters from kinetic config.
    ///
    /// Mirrors C++ `emMouseZoomScrollVIF::SetWheelAnimParams`.
    /// Same parameters as `set_mouse_anim_params` but uses a different
    /// spring constant numerator (480 vs 2500).
    pub fn set_wheel_anim_params(
        &mut self,
        kinetic_factor: f64,
        min_kinetic: f64,
        zoom_factor_log_per_pixel: f64,
    ) {
        let mut k = kinetic_factor;
        if k < min_kinetic * 1.0001 {
            k = 0.001;
        }
        let zflpp = zoom_factor_log_per_pixel.max(1e-10);
        self.wheel_spring_const = 480.0 / (k * k);
        self.wheel_friction = 2.0 / zflpp / (k * k);
        self.wheel_friction_enabled = k > 0.001;
    }

    /// Returns the wheel animator parameters (spring_const, friction, friction_enabled).
    pub fn wheel_anim_params(&self) -> (f64, f64, bool) {
        (
            self.wheel_spring_const,
            self.wheel_friction,
            self.wheel_friction_enabled,
        )
    }
}

impl Default for MouseZoomScrollVIF {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewInputFilter for MouseZoomScrollVIF {
    fn filter(&mut self, event: &InputEvent, state: &InputState, view: &mut View) -> bool {
        if view.flags.contains(ViewFlags::NO_NAVIGATE) {
            return false;
        }

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

        // Wheel zoom
        if matches!(event.key, InputKey::WheelUp | InputKey::WheelDown)
            && event.variant == InputVariant::Press
        {
            let down = event.key == InputKey::WheelDown;
            let factor = if down {
                1.0 / self.zoom_speed
            } else {
                self.zoom_speed
            };
            view.zoom(factor, state.mouse_x, state.mouse_y);
            return true;
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
    /// State machine for three-step programmatic navigation.
    /// 0 = waiting for Shift+Alt+End, 1 = waiting for letter,
    /// 2..27 = waiting for direction key (step = state - 1).
    nav_by_prog_state: u8,
}

impl KeyboardZoomScrollVIF {
    pub fn new() -> Self {
        Self {
            scroll_speed: 50.0,
            zoom_speed: 1.2,
            nav_by_prog_state: 0,
        }
    }

    /// Implement a three-step key sequence for programmatic navigation.
    ///
    /// Mirrors C++ `emKeyboardZoomScrollVIF::NavigateByProgram`.
    /// 1. Shift+Alt+End initiates (enters state 1).
    /// 2. Shift+Alt+A-Z selects step strength (enters state 2..27).
    /// 3. Shift+Alt+Arrow/Page executes scroll or zoom.
    ///
    /// Returns true if the event was consumed.
    pub fn navigate_by_program(
        &mut self,
        event: &InputEvent,
        state: &InputState,
        view: &mut View,
    ) -> bool {
        const SCROLL_DELTA: f64 = 0.3;
        const ZOOM_FAC: f64 = 1.0015;

        match self.nav_by_prog_state {
            0 => {
                // State 0: wait for Shift+Alt+End
                if event.key == InputKey::End
                    && event.variant == InputVariant::Press
                    && state.shift()
                    && state.alt()
                {
                    self.nav_by_prog_state = 1;
                    return true;
                }
                false
            }
            1 => {
                // State 1: wait for a letter key to determine step strength
                if event.variant != InputVariant::Press && event.variant != InputVariant::Repeat {
                    return false;
                }
                self.nav_by_prog_state = 0;

                if state.shift() && state.alt() {
                    // Compute step from key code: A=1, B=2, ..., Z=26
                    let step = match event.key {
                        InputKey::Key(c) => {
                            let upper = c.to_ascii_uppercase();
                            if upper.is_ascii_uppercase() {
                                upper as u8 - b'A' + 1
                            } else {
                                return false;
                            }
                        }
                        _ => return false,
                    };
                    if (1..=26).contains(&step) {
                        self.nav_by_prog_state = 1 + step;
                        return true;
                    }
                }
                false
            }
            s if s >= 2 => {
                // State 2..27: execute the navigation command
                if event.variant != InputVariant::Press && event.variant != InputVariant::Repeat {
                    return false;
                }
                let step = (s - 1) as f64;
                self.nav_by_prog_state = 0;

                if !state.shift() || !state.alt() {
                    return false;
                }

                let (vw, vh) = view.viewport_size();
                let cpt = (vh / vw.max(1.0)).max(0.001);

                match event.key {
                    InputKey::ArrowLeft => {
                        view.scroll(-SCROLL_DELTA * step * vw, 0.0);
                        true
                    }
                    InputKey::ArrowRight => {
                        view.scroll(SCROLL_DELTA * step * vw, 0.0);
                        true
                    }
                    InputKey::ArrowUp => {
                        view.scroll(0.0, -SCROLL_DELTA * step * vh / cpt);
                        true
                    }
                    InputKey::ArrowDown => {
                        view.scroll(0.0, SCROLL_DELTA * step * vh / cpt);
                        true
                    }
                    InputKey::PageUp => {
                        let factor = ZOOM_FAC.powf(step);
                        view.zoom(factor, vw * 0.5, vh * 0.5);
                        true
                    }
                    InputKey::PageDown => {
                        let factor = 1.0 / ZOOM_FAC.powf(step);
                        view.zoom(factor, vw * 0.5, vh * 0.5);
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

impl Default for KeyboardZoomScrollVIF {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewInputFilter for KeyboardZoomScrollVIF {
    fn filter(&mut self, event: &InputEvent, state: &InputState, view: &mut View) -> bool {
        if view.flags.contains(ViewFlags::NO_NAVIGATE) {
            return false;
        }

        // Try programmatic navigation first
        if self.navigate_by_program(event, state, view) {
            return true;
        }

        if event.variant != InputVariant::Press && event.variant != InputVariant::Repeat {
            return false;
        }

        match event.key {
            // Arrow keys require Alt modifier (matches C++ emDefaultTouchVIF)
            InputKey::ArrowUp if state.alt() => {
                view.scroll(0.0, -self.scroll_speed);
                true
            }
            InputKey::ArrowDown if state.alt() => {
                view.scroll(0.0, self.scroll_speed);
                true
            }
            InputKey::ArrowLeft if state.alt() => {
                view.scroll(-self.scroll_speed, 0.0);
                true
            }
            InputKey::ArrowRight if state.alt() => {
                view.scroll(self.scroll_speed, 0.0);
                true
            }
            // PageUp/Down zoom (matches C++ Alt requirement)
            InputKey::PageUp if state.alt() => {
                let (vw, vh) = view.viewport_size();
                view.zoom(self.zoom_speed, vw * 0.5, vh * 0.5);
                true
            }
            InputKey::PageDown if state.alt() => {
                let (vw, vh) = view.viewport_size();
                view.zoom(1.0 / self.zoom_speed, vw * 0.5, vh * 0.5);
                true
            }
            InputKey::Home if state.alt() => {
                view.go_home();
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panel::PanelTree;

    fn setup() -> (PanelTree, View) {
        let mut tree = PanelTree::new();
        let root = tree.create_root("root");
        tree.set_layout_rect(root, 0.0, 0.0, 1.0, 1.0);
        let view = View::new(root, 800.0, 600.0);
        (tree, view)
    }

    #[test]
    fn test_emulate_middle_button() {
        let mut vif = MouseZoomScrollVIF::new();
        let state = InputState::new();

        // Disabled by default
        let event = InputEvent::press(InputKey::Alt);
        assert!(vif
            .emulate_middle_button_event(&event, &state, 100)
            .is_none());

        // Enable and test — first press at 1000ms (well past initial time 0)
        vif.set_emulate_middle_button(true);
        let result = vif.emulate_middle_button_event(&event, &state, 1000);
        assert!(result.is_some());
        let synth = result.unwrap();
        assert_eq!(synth.key, InputKey::MouseMiddle);
        assert_eq!(synth.variant, InputVariant::Press);
        assert!(!synth.is_repeat);

        // Double-click within 330ms
        let event2 = InputEvent::press(InputKey::Alt);
        let result2 = vif.emulate_middle_button_event(&event2, &state, 1200);
        assert!(result2.is_some());
        assert!(result2.unwrap().is_repeat);

        // After timeout, repeat resets
        let event3 = InputEvent::press(InputKey::Alt);
        let result3 = vif.emulate_middle_button_event(&event3, &state, 2000);
        assert!(result3.is_some());
        assert!(!result3.unwrap().is_repeat);
    }

    #[test]
    fn test_update_wheel_zoom_speed() {
        let mut vif = MouseZoomScrollVIF::new();

        // Basic zoom in
        vif.update_wheel_zoom_speed(false, false, 1000, 0.0, 0.0);
        assert!(vif.wheel_zoom_speed() > 0.0);

        // Zoom out negates
        vif.update_wheel_zoom_speed(true, false, 1100, 0.0, 0.0);
        assert!(vif.wheel_zoom_speed() < 0.0);

        // Fine mode reduces speed
        vif.update_wheel_zoom_speed(false, true, 1200, 0.0, 0.0);
        let fine_speed = vif.wheel_zoom_speed();
        vif.update_wheel_zoom_speed(false, false, 1300, 0.0, 0.0);
        let normal_speed = vif.wheel_zoom_speed();
        assert!(fine_speed.abs() < normal_speed.abs());

        // Acceleration curve
        vif.update_wheel_zoom_speed(false, false, 2000, 5.0, 1.0);
        let accel_speed = vif.wheel_zoom_speed();
        assert!(accel_speed.abs() > 0.0);
    }

    #[test]
    fn test_set_mouse_anim_params() {
        let mut vif = MouseZoomScrollVIF::new();

        vif.set_mouse_anim_params(1.0, 0.5, 0.01);
        let (sc, fr, enabled) = vif.mouse_anim_params();
        assert!((sc - 2500.0).abs() < 0.1);
        assert!(fr > 0.0);
        assert!(enabled);

        // At minimum kinetic, clamps to 0.001
        vif.set_mouse_anim_params(0.5, 0.5, 0.01);
        let (sc2, _fr2, enabled2) = vif.mouse_anim_params();
        assert!(sc2 > 1e6); // 2500/(0.001^2) = very large
        assert!(!enabled2);
    }

    #[test]
    fn test_set_wheel_anim_params() {
        let mut vif = MouseZoomScrollVIF::new();

        vif.set_wheel_anim_params(1.0, 0.5, 0.01);
        let (sc, fr, enabled) = vif.wheel_anim_params();
        assert!((sc - 480.0).abs() < 0.1);
        assert!(fr > 0.0);
        assert!(enabled);
    }

    #[test]
    fn test_navigate_by_program() {
        let (mut tree, mut view) = setup();
        view.update_viewing(&mut tree);
        let mut vif = KeyboardZoomScrollVIF::new();

        let mut state = InputState::new();
        state.press(InputKey::Shift);
        state.press(InputKey::Alt);

        // Step 1: Shift+Alt+End
        let event = InputEvent::press(InputKey::End);
        assert!(vif.navigate_by_program(&event, &state, &mut view));

        // Step 2: Shift+Alt+C (step = 3)
        let event2 = InputEvent::press(InputKey::Key('c'));
        assert!(vif.navigate_by_program(&event2, &state, &mut view));

        // Step 3: Shift+Alt+Right (scroll right)
        let before = view.current_visit().rel_x;
        let event3 = InputEvent::press(InputKey::ArrowRight);
        assert!(vif.navigate_by_program(&event3, &state, &mut view));
        let after = view.current_visit().rel_x;
        assert!(after > before, "Should have scrolled right");
    }
}
