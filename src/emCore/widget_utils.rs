use crate::emCore::rect::Rect;

/// Returns true if TRACE_INPUT env var is set. Cached after first call.
pub(crate) fn trace_input_enabled() -> bool {
    use std::sync::OnceLock;
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("TRACE_INPUT").is_ok())
}

/// Rounded-rectangle hit test matching the C++ signed-distance formula used by
/// `emButton::CheckMouse`, `emTextField::CheckMouse`, and
/// `emScalarField::CheckMouse`.
///
/// Returns `true` when `(mx, my)` lies inside the rounded rectangle defined by
/// `rect` with corner radius `r`.
///
/// Formula: `dx = max(max(rx - mx, mx - rx - rw) + r, 0)`, same for dy,
/// then `hit = dx² + dy² ≤ r²`.
pub(crate) fn check_mouse_round_rect(mx: f64, my: f64, rect: &Rect, r: f64) -> bool {
    let dx = ((rect.x - mx).max(mx - rect.x - rect.w) + r).max(0.0);
    let dy = ((rect.y - my).max(my - rect.y - rect.h) + r).max(0.0);
    dx * dx + dy * dy <= r * r
}
