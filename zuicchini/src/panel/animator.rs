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
        let q = (1.0 - (-self.velocity_z * dt).exp()) / dt;
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
        let q = (1.0 - (-self.velocity_z * dt).exp()) / dt;
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

        // Apply linear friction per-dimension
        if self.friction_enabled {
            let a = self.friction;
            let abs_vel = (self.velocity_x * self.velocity_x
                + self.velocity_y * self.velocity_y
                + self.velocity_z * self.velocity_z)
                .sqrt();
            let f = if abs_vel > 0.0 {
                let reduced = abs_vel - a * dt;
                if reduced > 0.0 {
                    reduced / abs_vel
                } else {
                    0.0
                }
            } else {
                0.0
            };
            self.velocity_x *= f;
            self.velocity_y *= f;
            self.velocity_z *= f;
        }

        // Compute distances
        let dist = [
            self.velocity_x * dt,
            self.velocity_y * dt,
            self.velocity_z * dt,
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

/// Visiting view animator — smoothly animates the camera to a target visit state.
/// Uses logarithmic interpolation for zoom dimension.
pub struct VisitingViewAnimator {
    target_x: f64,
    target_y: f64,
    target_a: f64,
    speed: f64,
    active: bool,
}

impl VisitingViewAnimator {
    pub fn new(target_x: f64, target_y: f64, target_a: f64, speed: f64) -> Self {
        Self {
            target_x,
            target_y,
            target_a,
            speed,
            active: true,
        }
    }
}

impl ViewAnimator for VisitingViewAnimator {
    fn animate(&mut self, view: &mut View, tree: &mut PanelTree, dt: f64) -> bool {
        if !self.active {
            return false;
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

            // Check convergence
            if (new_x - self.target_x).abs() < 0.001
                && (new_y - self.target_y).abs() < 0.001
                && (new_log_a - log_target).abs() < 0.01
            {
                self.active = false;
            }
        }

        self.active
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn stop(&mut self) {
        self.active = false;
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
}
