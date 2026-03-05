use std::rc::Rc;

use crate::foundation::Color;

/// Theme configuration matching emLook's 10-color system.
pub struct Look {
    pub bg_color: Color,
    pub fg_color: Color,
    pub button_bg_color: Color,
    pub button_fg_color: Color,
    pub input_bg_color: Color,
    pub input_fg_color: Color,
    pub input_hl_color: Color,
    pub output_bg_color: Color,
    pub output_fg_color: Color,
    pub output_hl_color: Color,
}

impl Look {
    /// Create a new look wrapped in `Rc` with the default theme.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Border tint: bg_color darkened ~20%.
    pub fn border_tint(&self) -> Color {
        self.bg_color.darken(0.20)
    }

    /// Focus tint: same as input highlight color.
    pub fn focus_tint(&self) -> Color {
        self.input_hl_color
    }

    /// Disabled foreground: fg blended 50% toward bg.
    pub fn disabled_fg(&self) -> Color {
        self.fg_color.lerp(self.bg_color, 0.5)
    }

    /// Button hover: button_bg lightened ~15%.
    pub fn button_hover(&self) -> Color {
        self.button_bg_color.lighten(0.15)
    }

    /// Button pressed: button_bg darkened ~15%.
    pub fn button_pressed(&self) -> Color {
        self.button_bg_color.darken(0.15)
    }
}

impl Default for Look {
    fn default() -> Self {
        Self {
            bg_color: Color::rgba(0x51, 0x5E, 0x84, 0xFF),
            fg_color: Color::rgba(0xEF, 0xF0, 0xF4, 0xFF),
            button_bg_color: Color::rgba(0x59, 0x67, 0x90, 0xFF),
            button_fg_color: Color::rgba(0xF2, 0xF2, 0xF7, 0xFF),
            input_bg_color: Color::rgba(0xEF, 0xF0, 0xF4, 0xFF),
            input_fg_color: Color::rgba(0x02, 0x0E, 0x1D, 0xFF),
            input_hl_color: Color::rgba(0x00, 0x38, 0xC0, 0xFF),
            output_bg_color: Color::rgba(0xA7, 0xA9, 0xB0, 0xFF),
            output_fg_color: Color::rgba(0x07, 0x0B, 0x18, 0xFF),
            output_hl_color: Color::rgba(0x00, 0x2B, 0x9A, 0xFF),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_look_has_emlook_colors() {
        let look = Look::default();
        assert_eq!(look.bg_color, Color::rgba(0x51, 0x5E, 0x84, 0xFF));
        assert_eq!(look.fg_color, Color::rgba(0xEF, 0xF0, 0xF4, 0xFF));
        assert_eq!(look.input_hl_color, Color::rgba(0x00, 0x38, 0xC0, 0xFF));
    }

    #[test]
    fn derived_colors_are_reasonable() {
        let look = Look::default();
        // border_tint should be darker than bg
        let bt = look.border_tint();
        assert!(bt.r() < look.bg_color.r());
        // button_hover should be lighter than button_bg
        let bh = look.button_hover();
        assert!(bh.r() > look.button_bg_color.r());
    }
}
