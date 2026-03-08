use super::tree::PanelTree;
use super::view::View;

/// Trait for view animation strategies.
pub trait ViewAnimator {
    /// Advance the animation by one frame. Returns true if still animating.
    fn animate(&mut self, view: &mut View, tree: &mut PanelTree, dt: f64) -> bool;

    /// Whether the animation is currently active.
    fn is_active(&self) -> bool;

    /// Stop the animation immediately.
    fn stop(&mut self);
}

/// Kinetic view animator — applies velocity with linear friction for smooth deceleration.
/// Used for fling/swipe gestures. Supports 3D (scroll x, scroll y, zoom z).
pub struct KineticViewAnimator {
    velocity_x: f64,
    velocity_y: f64,
    velocity_z: f64,
    friction: f64,
    friction_enabled: bool,
    zoom_fix_point_centered: bool,
    zoom_fix_x: f64,
    zoom_fix_y: f64,
    active: bool,
}

impl KineticViewAnimator {
    pub fn new(velocity_x: f64, velocity_y: f64, velocity_z: f64, friction: f64) -> Self {
        Self {
            velocity_x,
            velocity_y,
            velocity_z,
            friction,
            friction_enabled: false,
            zoom_fix_point_centered: true,
            zoom_fix_x: 0.0,
            zoom_fix_y: 0.0,
            active: velocity_x.abs() > 0.01 || velocity_y.abs() > 0.01 || velocity_z.abs() > 0.01,
        }
    }

    pub fn set_velocity(&mut self, vx: f64, vy: f64, vz: f64) {
        self.velocity_x = vx;
        self.velocity_y = vy;
        self.velocity_z = vz;
        self.active = vx.abs() > 0.01 || vy.abs() > 0.01 || vz.abs() > 0.01;
    }

    pub fn velocity(&self) -> (f64, f64, f64) {
        (self.velocity_x, self.velocity_y, self.velocity_z)
    }

    pub fn set_friction_enabled(&mut self, enabled: bool) {
        self.friction_enabled = enabled;
    }

    pub fn is_friction_enabled(&self) -> bool {
        self.friction_enabled
    }

    pub fn set_friction(&mut self, friction: f64) {
        self.friction = friction;
    }

    pub fn friction(&self) -> f64 {
        self.friction
    }

    /// Switch zoom fix point to centered mode, compensating XY velocity.
    pub fn center_zoom_fix_point(&mut self, view: &View) {
        if self.zoom_fix_point_centered {
            return;
        }
        let old_fix_x = self.zoom_fix_x;
        let old_fix_y = self.zoom_fix_y;
        self.zoom_fix_point_centered = true;
        self.update_zoom_fix_point(view);
        let dt = 0.01;
        let zflpp = view.get_zoom_factor_log_per_pixel();
        let q = (1.0 - (-self.velocity_z * dt * zflpp).exp()) / dt;
        self.velocity_x += (old_fix_x - self.zoom_fix_x) * q;
        self.velocity_y += (old_fix_y - self.zoom_fix_y) * q;
    }

    /// Set an explicit (non-centered) zoom fix point, compensating XY velocity.
    pub fn set_zoom_fix_point(&mut self, x: f64, y: f64, view: &View) {
        if !self.zoom_fix_point_centered && self.zoom_fix_x == x && self.zoom_fix_y == y {
            return;
        }
        self.update_zoom_fix_point(view);
        let old_fix_x = self.zoom_fix_x;
        let old_fix_y = self.zoom_fix_y;
        self.zoom_fix_point_centered = false;
        self.zoom_fix_x = x;
        self.zoom_fix_y = y;
        let dt = 0.01;
        let zflpp = view.get_zoom_factor_log_per_pixel();
        let q = (1.0 - (-self.velocity_z * dt * zflpp).exp()) / dt;
        self.velocity_x += (old_fix_x - self.zoom_fix_x) * q;
        self.velocity_y += (old_fix_y - self.zoom_fix_y) * q;
    }

    /// If centered, update fix point to viewport center.
    pub fn update_zoom_fix_point(&mut self, view: &View) {
        if self.zoom_fix_point_centered {
            let (vw, vh) = view.viewport_size();
            self.zoom_fix_x = vw * 0.5;
            self.zoom_fix_y = vh * 0.5;
        }
    }

    fn update_busy_state(&mut self) {
        let abs_vel = (self.velocity_x * self.velocity_x
            + self.velocity_y * self.velocity_y
            + self.velocity_z * self.velocity_z)
            .sqrt();
        if self.active && abs_vel > 0.01 {
            // stay active
        } else {
            self.velocity_x = 0.0;
            self.velocity_y = 0.0;
            self.velocity_z = 0.0;
            self.active = false;
        }
    }
}

impl ViewAnimator for KineticViewAnimator {
    fn animate(&mut self, view: &mut View, tree: &mut PanelTree, dt: f64) -> bool {
        if !self.active {
            return false;
        }

        // Save pre-friction velocities for average displacement
        let vx_before = self.velocity_x;
        let vy_before = self.velocity_y;
        let vz_before = self.velocity_z;

        // Apply linear friction per-dimension independently
        if self.friction_enabled {
            let a = self.friction;
            self.velocity_x = apply_friction_1d(self.velocity_x, a, dt);
            self.velocity_y = apply_friction_1d(self.velocity_y, a, dt);
            self.velocity_z = apply_friction_1d(self.velocity_z, a, dt);
        }

        // Compute distances using average of pre/post-friction velocity
        let dist = [
            (vx_before + self.velocity_x) * 0.5 * dt,
            (vy_before + self.velocity_y) * 0.5 * dt,
            (vz_before + self.velocity_z) * 0.5 * dt,
        ];

        // Skip if motion is negligible
        if dist[0].abs() < 0.01 && dist[1].abs() < 0.01 && dist[2].abs() < 0.01 {
            self.update_busy_state();
            return self.active;
        }

        // Apply scroll and zoom
        self.update_zoom_fix_point(view);
        let done = view.raw_scroll_and_zoom(
            tree,
            self.zoom_fix_x,
            self.zoom_fix_y,
            dist[0],
            dist[1],
            dist[2],
        );

        // Blocked-motion feedback: zero velocity for blocked dimensions
        for i in 0..3 {
            if done[i].abs() < 0.99 * dist[i].abs() {
                match i {
                    0 => self.velocity_x = 0.0,
                    1 => self.velocity_y = 0.0,
                    2 => self.velocity_z = 0.0,
                    _ => unreachable!(),
                }
            }
        }

        self.update_busy_state();
        self.active
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn stop(&mut self) {
        self.velocity_x = 0.0;
        self.velocity_y = 0.0;
        self.velocity_z = 0.0;
        self.active = false;
    }
}

/// Speeding view animator — accelerates toward a target velocity.
/// Composes a KineticViewAnimator for scroll/zoom delegation.
/// Used for keyboard-driven scrolling. Supports 3D.
pub struct SpeedingViewAnimator {
    inner: KineticViewAnimator,
    target_vx: f64,
    target_vy: f64,
    target_vz: f64,
    acceleration: f64,
    reverse_acceleration: f64,
    active: bool,
}

impl SpeedingViewAnimator {
    pub fn new(friction: f64) -> Self {
        Self {
            inner: KineticViewAnimator::new(0.0, 0.0, 0.0, friction),
            target_vx: 0.0,
            target_vy: 0.0,
            target_vz: 0.0,
            acceleration: 1.0,
            reverse_acceleration: 1.0,
            active: false,
        }
    }

    pub fn set_target(&mut self, vx: f64, vy: f64, vz: f64) {
        self.target_vx = vx;
        self.target_vy = vy;
        self.target_vz = vz;
        self.active = true;
    }

    pub fn release(&mut self) {
        self.target_vx = 0.0;
        self.target_vy = 0.0;
        self.target_vz = 0.0;
    }

    pub fn set_acceleration(&mut self, accel: f64) {
        self.acceleration = accel;
    }

    pub fn set_reverse_acceleration(&mut self, accel: f64) {
        self.reverse_acceleration = accel;
    }

    pub fn inner(&self) -> &KineticViewAnimator {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut KineticViewAnimator {
        &mut self.inner
    }
}

/// 3-branch acceleration: reverse, forward, or friction deceleration.
fn accelerate_dim(
    v: f64,
    target: f64,
    accel: f64,
    reverse_accel: f64,
    friction: f64,
    friction_enabled: bool,
    dt: f64,
) -> f64 {
    let adt = if v * target < -0.1 {
        // Opposite direction — use reverse acceleration
        reverse_accel * dt
    } else if v.abs() < target.abs() {
        // Below target speed — use forward acceleration, clamp dt
        accel * dt.min(0.1)
    } else if friction_enabled {
        // Above target speed — use friction deceleration
        friction * dt
    } else {
        0.0
    };

    if v - adt > target {
        v - adt
    } else if v + adt < target {
        v + adt
    } else {
        target
    }
}

impl ViewAnimator for SpeedingViewAnimator {
    fn animate(&mut self, view: &mut View, tree: &mut PanelTree, dt: f64) -> bool {
        if !self.active {
            return false;
        }

        // 3-branch acceleration per dimension
        let (vx, vy, vz) = self.inner.velocity();
        let friction = self.inner.friction();
        let friction_enabled = self.inner.is_friction_enabled();

        let new_vx = accelerate_dim(
            vx,
            self.target_vx,
            self.acceleration,
            self.reverse_acceleration,
            friction,
            friction_enabled,
            dt,
        );
        let new_vy = accelerate_dim(
            vy,
            self.target_vy,
            self.acceleration,
            self.reverse_acceleration,
            friction,
            friction_enabled,
            dt,
        );
        let new_vz = accelerate_dim(
            vz,
            self.target_vz,
            self.acceleration,
            self.reverse_acceleration,
            friction,
            friction_enabled,
            dt,
        );
        self.inner.set_velocity(new_vx, new_vy, new_vz);

        // Temporarily disable friction on inner (speeding handles it via acceleration)
        let saved_friction = self.inner.is_friction_enabled();
        self.inner.set_friction_enabled(false);
        self.inner.animate(view, tree, dt);
        self.inner.set_friction_enabled(saved_friction);

        // Idle check: target near zero and inner stopped
        if self.target_vx.abs() < 0.01
            && self.target_vy.abs() < 0.01
            && self.target_vz.abs() < 0.01
            && !self.inner.is_active()
        {
            self.active = false;
        }

        self.active
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn stop(&mut self) {
        self.inner.stop();
        self.target_vx = 0.0;
        self.target_vy = 0.0;
        self.target_vz = 0.0;
        self.active = false;
    }
}

/// State for the visiting animator's seek/navigation progress.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum VisitingState {
    /// No goal set.
    NoGoal,
    /// Animating along a curved path.
    Curve,
    /// Animating directly toward the target.
    Direct,
    /// Seeking: waiting for panels to be lazily created.
    Seek,
    /// Seek failed, showing "Not found" overlay briefly.
    GivingUp,
    /// Terminal: gave up after showing overlay.
    GivenUp,
    /// Terminal: goal reached.
    GoalReached,
}

/// Visiting view animator — smoothly animates the camera to a target visit state.
/// Uses logarithmic interpolation for zoom dimension.
pub struct VisitingViewAnimator {
    target_x: f64,
    target_y: f64,
    target_a: f64,
    speed: f64,
    active: bool,
    /// Whether smooth animation is enabled (false = instant visit).
    animated: bool,
    /// Acceleration for speed ramping (units/s^2).
    acceleration: f64,
    /// Maximum speed at zoom cusp (zoom-out-then-in transition).
    max_cusp_speed: f64,
    /// Maximum absolute animation speed.
    max_absolute_speed: f64,
    /// Current state in the seek/navigation state machine.
    state: VisitingState,
    /// Target panel identity string (slash-separated path).
    identity: String,
    /// Human-readable subject description for seek overlay.
    subject: String,
}

impl VisitingViewAnimator {
    pub fn new(target_x: f64, target_y: f64, target_a: f64, speed: f64) -> Self {
        Self {
            target_x,
            target_y,
            target_a,
            speed,
            active: true,
            animated: false,
            acceleration: 5.0,
            max_cusp_speed: 2.0,
            max_absolute_speed: 5.0,
            state: VisitingState::Curve,
            identity: String::new(),
            subject: String::new(),
        }
    }

    /// Configure animation parameters from a speed config value.
    ///
    /// Mirrors C++ `emVisitingViewAnimator::SetAnimParamsByCoreConfig`.
    /// `speed_factor` is the user's configured visit speed (typically 0..max).
    /// `max_speed_factor` is the maximum value of that config range.
    ///
    /// When `speed_factor` is near `max_speed_factor`, animation is disabled
    /// (instant visit). Otherwise, acceleration and max speeds are scaled by
    /// `35.0 * speed_factor`, and cusp speed is half of max absolute speed.
    pub fn set_anim_params_by_speed_config(&mut self, speed_factor: f64, max_speed_factor: f64) {
        self.animated = speed_factor < max_speed_factor * 0.99999;
        self.acceleration = 35.0 * speed_factor;
        self.max_absolute_speed = 35.0 * speed_factor;
        self.max_cusp_speed = self.max_absolute_speed * 0.5;
    }

    /// Returns the current visiting state.
    pub(crate) fn visiting_state(&self) -> VisitingState {
        self.state
    }

    /// Set state (used by seek logic / tests).
    pub(crate) fn set_visiting_state(&mut self, state: VisitingState) {
        self.state = state;
    }

    /// Set the identity and subject for seek overlay display.
    pub fn set_identity(&mut self, identity: &str, subject: &str) {
        self.identity = identity.to_string();
        self.subject = subject.to_string();
    }

    /// Returns the identity string being visited.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Handle input during visiting animation.
    ///
    /// Mirrors C++ `emVisitingViewAnimator::Input`.
    /// During seek or giving-up states, any key/mouse event aborts the
    /// seek and deactivates the animator. Returns true if the event was
    /// consumed (eaten).
    pub fn handle_input(&mut self, event: &crate::input::InputEvent) -> bool {
        if !self.active {
            return false;
        }
        if self.state != VisitingState::Seek && self.state != VisitingState::GivingUp {
            return false;
        }
        // Any non-empty event aborts the seek
        if event.key != crate::input::InputKey::MouseLeft
            || event.variant != crate::input::InputVariant::Move
        {
            // An actual key/button event (not just mouse move) — abort
            self.active = false;
            self.state = VisitingState::GivenUp;
            return true;
        }
        false
    }

    /// Paint the seek progress overlay.
    ///
    /// Mirrors C++ `emVisitingViewAnimator::Paint`.
    /// Shows a semi-transparent overlay with the target identity and a
    /// progress bar when in Seek or GivingUp state.
    pub fn paint_seek_overlay(&self, painter: &mut crate::render::Painter<'_>, view: &View) {
        if !self.active {
            return;
        }
        if self.state != VisitingState::Seek && self.state != VisitingState::GivingUp {
            return;
        }

        let (vw, vh) = view.viewport_size();
        let w = (vw.max(vh) * 0.6).min(vw);
        let mut h = w * 0.25;

        let f = vh * 0.8 / h;
        if f < 1.0 {
            h *= f;
        }

        let x = (vw - w) * 0.5;
        let y = (vh - h) * 0.5;

        // Shadow
        let shadow_off = w * 0.03;
        painter.paint_round_rect(
            x + shadow_off,
            y + shadow_off,
            w,
            h,
            h * 0.2,
            crate::foundation::Color::rgba(0, 0, 0, 160),
        );

        // Background box
        painter.paint_round_rect(
            x,
            y,
            w,
            h,
            h * 0.2,
            crate::foundation::Color::rgba(34, 102, 153, 208),
        );

        let ch = h * 0.22;

        if self.state == VisitingState::GivingUp {
            // "Not found" text
            painter.paint_text(
                x + w * 0.1,
                y + h * 0.15,
                "Not found",
                ch,
                crate::foundation::Color::rgba(255, 136, 136, 255),
            );
            return;
        }

        // "Seeking..." text
        let mut seeking_text = String::from("Seeking...");
        if !self.subject.is_empty() {
            seeking_text.push_str(" for ");
            seeking_text.push_str(&self.subject);
        }
        painter.paint_text(
            x + w * 0.05,
            y + h * 0.1,
            &seeking_text,
            ch * 0.7,
            crate::foundation::Color::rgba(221, 221, 221, 255),
        );

        // Progress bar background
        let bar_x = x + w * 0.05;
        let bar_y = y + h * 0.45;
        let bar_w = w * 0.9;
        let bar_h = h * 0.15;

        // Compute progress from identity match
        let seek_id = view
            .seek_pos_panel()
            .map(|_| view.seek_pos_child_name())
            .unwrap_or("");
        let total_len = self.identity.len().max(1);
        let found_len = if !seek_id.is_empty() {
            self.identity
                .find(seek_id)
                .map(|pos| pos + seek_id.len())
                .unwrap_or(0)
                .min(total_len)
        } else {
            0
        };
        let progress = found_len as f64 / total_len as f64;

        // Found portion (green)
        if progress > 0.0 {
            painter.paint_rect(
                bar_x,
                bar_y,
                bar_w * progress,
                bar_h,
                crate::foundation::Color::rgba(136, 255, 136, 80),
            );
        }
        // Remaining portion (gray)
        if progress < 1.0 {
            painter.paint_rect(
                bar_x + bar_w * progress,
                bar_y,
                bar_w * (1.0 - progress),
                bar_h,
                crate::foundation::Color::rgba(136, 136, 136, 80),
            );
        }

        // Identity text
        painter.paint_text(
            bar_x,
            bar_y + bar_h + h * 0.05,
            &self.identity,
            ch * 0.5,
            crate::foundation::Color::rgba(221, 221, 221, 255),
        );

        // Abort instruction
        painter.paint_text(
            x + w * 0.05,
            y + h * 0.8,
            "Press any key to abort",
            ch * 0.5,
            crate::foundation::Color::rgba(170, 170, 170, 255),
        );
    }
}

impl ViewAnimator for VisitingViewAnimator {
    fn animate(&mut self, view: &mut View, tree: &mut PanelTree, dt: f64) -> bool {
        if !self.active {
            return false;
        }

        match self.visiting_state() {
            VisitingState::NoGoal | VisitingState::GivenUp | VisitingState::GoalReached => {
                return false;
            }
            VisitingState::GivingUp => {
                // Still showing "Not found" — just stay active
                return true;
            }
            VisitingState::Seek => {
                // Seek state not fully implemented; fall through to animate
            }
            VisitingState::Curve | VisitingState::Direct => {
                // Continue with animation below
            }
        }

        let t = (self.speed * dt).min(1.0);

        if let Some(state) = view.visit_stack().last().cloned() {
            let new_x = lerp(state.rel_x, self.target_x, t);
            let new_y = lerp(state.rel_y, self.target_y, t);
            // Logarithmic interpolation for zoom
            let log_a = state.rel_a.ln();
            let log_target = self.target_a.max(0.001).ln();
            let new_log_a = lerp(log_a, log_target, t);
            let new_a = new_log_a.exp();

            let dx = (new_x - state.rel_x) * view.viewport_size().0.max(1.0);
            let dy = (new_y - state.rel_y) * view.viewport_size().1.max(1.0);
            let dz = if state.rel_a > 0.0 {
                (new_a / state.rel_a).ln()
            } else {
                0.0
            };

            let (vw, vh) = view.viewport_size();
            view.raw_scroll_and_zoom(tree, vw * 0.5, vh * 0.5, dx, dy, dz);

            // Transition from Curve to Direct when close enough
            if self.visiting_state() == VisitingState::Curve {
                let dist = (new_x - self.target_x).abs()
                    + (new_y - self.target_y).abs()
                    + (new_log_a - log_target).abs();
                if dist < 0.1 {
                    self.set_visiting_state(VisitingState::Direct);
                }
            }

            // Check convergence
            if (new_x - self.target_x).abs() < 0.001
                && (new_y - self.target_y).abs() < 0.001
                && (new_log_a - log_target).abs() < 0.01
            {
                self.active = false;
                self.set_visiting_state(VisitingState::GoalReached);
            }
        }

        self.active
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn stop(&mut self) {
        self.active = false;
        self.set_visiting_state(VisitingState::NoGoal);
    }
}

/// Per-dimension linear friction: reduces signed velocity toward zero by `a * dt`.
fn apply_friction_1d(v: f64, a: f64, dt: f64) -> f64 {
    if v - a * dt > 0.0 {
        v - a * dt
    } else if v + a * dt < 0.0 {
        v + a * dt
    } else {
        0.0
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
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
    fn kinetic_with_zoom() {
        let (mut tree, mut view) = setup();
        view.update_viewing(&mut tree);
        let initial_a = view.current_visit().rel_a;

        let mut anim = KineticViewAnimator::new(0.0, 0.0, 1.0, 1000.0);
        // friction_enabled defaults to false — just test that zoom scroll works
        anim.animate(&mut view, &mut tree, 0.1);

        // Zoom velocity should have changed rel_a
        assert!((view.current_visit().rel_a - initial_a).abs() > 0.001);
    }

    #[test]
    fn speeding_with_zoom() {
        let (mut tree, mut view) = setup();
        view.update_viewing(&mut tree);

        let mut anim = SpeedingViewAnimator::new(1000.0);
        anim.set_target(0.0, 0.0, 2.0);

        for _ in 0..10 {
            anim.animate(&mut view, &mut tree, 0.016);
        }

        // Should be accelerating toward zoom
        let (_, _, vz) = anim.inner().velocity();
        assert!(vz.abs() > 0.0);
    }

    #[test]
    fn visiting_converges() {
        let (mut tree, mut view) = setup();
        view.update_viewing(&mut tree);

        let mut anim = VisitingViewAnimator::new(0.1, 0.1, 2.0, 10.0);

        for _ in 0..100 {
            if !anim.animate(&mut view, &mut tree, 0.016) {
                break;
            }
        }

        assert!(!anim.is_active());
    }

    #[test]
    fn kinetic_linear_friction_stops() {
        let (mut tree, mut view) = setup();
        view.update_viewing(&mut tree);

        let mut anim = KineticViewAnimator::new(100.0, 0.0, 0.0, 1000.0);
        anim.set_friction_enabled(true);

        for _ in 0..200 {
            if !anim.animate(&mut view, &mut tree, 0.016) {
                break;
            }
        }

        assert!(!anim.is_active());
    }

    #[test]
    fn kinetic_friction_disabled() {
        let (mut tree, mut view) = setup();
        view.update_viewing(&mut tree);

        let mut anim = KineticViewAnimator::new(100.0, 0.0, 0.0, 1000.0);
        // friction_enabled defaults to false

        anim.animate(&mut view, &mut tree, 0.016);

        let (vx, _, _) = anim.velocity();
        // Without friction, velocity should remain at 100.0 (or zeroed by blocked-motion)
        // but should NOT have been reduced by friction
        assert!(vx == 100.0 || vx == 0.0);
    }

    #[test]
    fn speeding_3branch_reverse() {
        let (mut tree, mut view) = setup();
        view.update_viewing(&mut tree);

        let mut anim = SpeedingViewAnimator::new(1000.0);
        anim.set_reverse_acceleration(500.0);

        // Set inner velocity going right (set_velocity activates if > 0.01)
        anim.inner_mut().set_velocity(100.0, 0.0, 0.0);
        // Target going left — should trigger reverse acceleration
        anim.set_target(-100.0, 0.0, 0.0);

        anim.animate(&mut view, &mut tree, 0.016);

        let (vx, _, _) = anim.inner().velocity();
        // Velocity should have moved toward -100 (decreased from 100)
        assert!(vx < 100.0);
    }

    #[test]
    fn speeding_delegates_to_kinetic() {
        let (mut tree, mut view) = setup();
        view.update_viewing(&mut tree);
        let initial_a = view.current_visit().rel_a;

        let mut anim = SpeedingViewAnimator::new(1000.0);
        anim.set_target(0.0, 0.0, 2.0);
        anim.set_acceleration(1000.0);

        for _ in 0..10 {
            anim.animate(&mut view, &mut tree, 0.016);
        }

        // Inner kinetic should have applied zoom via raw_scroll_and_zoom
        assert!((view.current_visit().rel_a - initial_a).abs() > 0.001);
    }

    #[test]
    fn visiting_set_anim_params() {
        let mut anim = VisitingViewAnimator::new(0.0, 0.0, 1.0, 5.0);

        // Below max: animated
        anim.set_anim_params_by_speed_config(2.0, 10.0);
        assert!(anim.animated);
        assert!((anim.acceleration - 70.0).abs() < 0.01);
        assert!((anim.max_absolute_speed - 70.0).abs() < 0.01);
        assert!((anim.max_cusp_speed - 35.0).abs() < 0.01);

        // At max: not animated (instant)
        anim.set_anim_params_by_speed_config(10.0, 10.0);
        assert!(!anim.animated);
    }

    #[test]
    fn visiting_handle_input_abort() {
        let mut anim = VisitingViewAnimator::new(0.0, 0.0, 1.0, 5.0);

        // Not in seek state — should not consume
        let event = crate::input::InputEvent::press(crate::input::InputKey::Escape);
        assert!(!anim.handle_input(&event));

        // Set to seek state
        anim.set_visiting_state(VisitingState::Seek);
        assert!(anim.handle_input(&event));
        assert!(!anim.is_active());
        assert_eq!(anim.visiting_state(), VisitingState::GivenUp);
    }

    #[test]
    fn visiting_state_direct_transitions() {
        let mut anim = VisitingViewAnimator::new(0.0, 0.0, 1.0, 5.0);

        // Exercise all state variants to ensure they exist
        anim.set_visiting_state(VisitingState::NoGoal);
        assert_eq!(anim.visiting_state(), VisitingState::NoGoal);

        anim.set_visiting_state(VisitingState::Direct);
        assert_eq!(anim.visiting_state(), VisitingState::Direct);

        anim.set_visiting_state(VisitingState::GivingUp);
        assert_eq!(anim.visiting_state(), VisitingState::GivingUp);

        anim.set_visiting_state(VisitingState::GoalReached);
        assert_eq!(anim.visiting_state(), VisitingState::GoalReached);
    }
}
