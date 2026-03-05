use std::rc::Rc;

use crate::foundation::Color;

/// Theme configuration for widget rendering.
pub struct Look {
    pub bg_color: Color,
    pub fg_color: Color,
    pub button_color: Color,
    pub button_hover_color: Color,
    pub button_press_color: Color,
    pub input_bg_color: Color,
    pub input_border_color: Color,
    pub output_bg_color: Color,
    pub selection_color: Color,
    pub border_color: Color,
    pub group_border_color: Color,
    pub focus_color: Color,
    pub disabled_fg_color: Color,
    pub check_color: Color,
    pub cursor_color: Color,
    pub scale_mark_color: Color,
}

impl Look {
    /// Create a new look wrapped in `Rc` with the default dark theme.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Create a light theme variant wrapped in `Rc`.
    pub fn light() -> Rc<Self> {
        Rc::new(Self {
            bg_color: Color::rgb(240, 240, 240),
            fg_color: Color::rgb(20, 20, 20),
            button_color: Color::rgb(210, 210, 210),
            button_hover_color: Color::rgb(225, 225, 225),
            button_press_color: Color::rgb(180, 180, 180),
            input_bg_color: Color::rgb(255, 255, 255),
            input_border_color: Color::rgb(160, 160, 160),
            output_bg_color: Color::rgb(245, 245, 245),
            selection_color: Color::rgb(50, 120, 200),
            border_color: Color::rgb(180, 180, 180),
            group_border_color: Color::rgb(200, 200, 200),
            focus_color: Color::rgb(50, 120, 200),
            disabled_fg_color: Color::rgb(160, 160, 160),
            check_color: Color::rgb(50, 120, 200),
            cursor_color: Color::rgb(20, 20, 20),
            scale_mark_color: Color::rgb(140, 140, 140),
        })
    }
}

impl Default for Look {
    fn default() -> Self {
        Self {
            bg_color: Color::rgb(50, 50, 55),
            fg_color: Color::rgb(210, 210, 210),
            button_color: Color::rgb(80, 80, 88),
            button_hover_color: Color::rgb(100, 100, 110),
            button_press_color: Color::rgb(60, 60, 66),
            input_bg_color: Color::rgb(30, 30, 34),
            input_border_color: Color::rgb(90, 90, 100),
            output_bg_color: Color::rgb(40, 40, 44),
            selection_color: Color::rgb(50, 90, 160),
            border_color: Color::rgb(70, 70, 78),
            group_border_color: Color::rgb(80, 80, 88),
            focus_color: Color::rgb(70, 130, 210),
            disabled_fg_color: Color::rgb(100, 100, 108),
            check_color: Color::rgb(70, 160, 220),
            cursor_color: Color::rgb(210, 210, 210),
            scale_mark_color: Color::rgb(90, 90, 100),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_look_creates_dark_theme() {
        let look = Look::new();
        assert_eq!(look.bg_color, Color::rgb(50, 50, 55));
        assert_eq!(look.fg_color, Color::rgb(210, 210, 210));
    }

    #[test]
    fn light_look_creates_light_theme() {
        let look = Look::light();
        assert_eq!(look.bg_color, Color::rgb(240, 240, 240));
        assert_eq!(look.fg_color, Color::rgb(20, 20, 20));
    }
}
