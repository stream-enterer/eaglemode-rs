// SPLIT: emFileManTheme.h — emFileManThemeNames split into separate file per one-type-per-file rule.

use std::collections::BTreeMap;

struct ThemeAR {
    name: String,
    aspect_ratio: String,
    height: f64,
}

struct ThemeStyle {
    display_name: String,
    display_icon: String,
    theme_ars: Vec<ThemeAR>,
}

pub struct emFileManThemeNames {
    styles: Vec<ThemeStyle>,
    name_to_packed_index: BTreeMap<String, (usize, usize)>,
}

/// Port of C++ `emFileManThemeNames::HeightToAspectRatioString`.
/// Tries denominators 1..10, finds best n:d ratio for the given height.
pub fn HeightToAspectRatioString(height: f64) -> String {
    let mut best_n: i32 = 1;
    let mut best_d: i32 = 1;
    for d in 1..=10_i32 {
        let mut n = (f64::from(d) / height + 0.5) as i32;
        if n < 1 {
            n = 1;
        }
        if (height * f64::from(n) / f64::from(d) - 1.0).abs()
            < (height * f64::from(best_n) / f64::from(best_d) - 1.0).abs() - 0.001
        {
            best_n = n;
            best_d = d;
        }
    }
    format!("{best_n}:{best_d}")
}

impl emFileManThemeNames {
    /// Construct from a slice of `(name, display_name, display_icon, height)` tuples.
    /// This mirrors the C++ constructor logic but takes pre-parsed theme data for testability.
    pub fn from_themes(entries: &[(&str, &str, &str, f64)]) -> Self {
        let mut sorted_entries: Vec<_> = entries.to_vec();
        sorted_entries.sort_by(|a, b| a.0.cmp(b.0));

        let mut styles: Vec<ThemeStyle> = Vec::new();
        let mut name_to_packed_index: BTreeMap<String, (usize, usize)> = BTreeMap::new();

        for &(name, display_name, display_icon, height) in &sorted_entries {
            let aspect_ratio = HeightToAspectRatioString(height);

            // Find existing style by display_name, or create new one
            let style_idx = styles
                .iter()
                .position(|s| s.display_name == display_name)
                .unwrap_or_else(|| {
                    styles.push(ThemeStyle {
                        display_name: display_name.to_string(),
                        display_icon: display_icon.to_string(),
                        theme_ars: Vec::new(),
                    });
                    styles.len() - 1
                });

            // Insert sorted by height within the style
            let style = &mut styles[style_idx];
            let ar_idx = style
                .theme_ars
                .iter()
                .position(|ar| ar.height > height)
                .unwrap_or(style.theme_ars.len());

            style.theme_ars.insert(
                ar_idx,
                ThemeAR {
                    name: name.to_string(),
                    aspect_ratio,
                    height,
                },
            );

            name_to_packed_index.insert(name.to_string(), (style_idx, ar_idx));
        }

        // Rebuild packed index after all insertions (insertion offsets may have shifted)
        name_to_packed_index.clear();
        for (style_idx, style) in styles.iter().enumerate() {
            for (ar_idx, ar) in style.theme_ars.iter().enumerate() {
                name_to_packed_index.insert(ar.name.clone(), (style_idx, ar_idx));
            }
        }

        Self {
            styles,
            name_to_packed_index,
        }
    }

    pub fn GetThemeStyleCount(&self) -> usize {
        self.styles.len()
    }

    pub fn GetThemeAspectRatioCount(&self, style_index: usize) -> usize {
        self.styles
            .get(style_index)
            .map_or(0, |s| s.theme_ars.len())
    }

    pub fn GetThemeName(&self, style_index: usize, ar_index: usize) -> Option<String> {
        self.styles
            .get(style_index)
            .and_then(|s| s.theme_ars.get(ar_index))
            .map(|ar| ar.name.clone())
    }

    pub fn GetDefaultThemeName(&self) -> String {
        if self.IsExistingThemeName("Glass1") {
            return "Glass1".to_string();
        }
        self.GetThemeName(0, 0).unwrap_or_default()
    }

    pub fn GetThemeStyleDisplayName(&self, style_index: usize) -> Option<&str> {
        self.styles.get(style_index).map(|s| s.display_name.as_str())
    }

    pub fn GetThemeStyleDisplayIcon(&self, style_index: usize) -> Option<&str> {
        self.styles.get(style_index).map(|s| s.display_icon.as_str())
    }

    pub fn GetThemeAspectRatio(&self, style_index: usize, ar_index: usize) -> Option<&str> {
        self.styles
            .get(style_index)
            .and_then(|s| s.theme_ars.get(ar_index))
            .map(|ar| ar.aspect_ratio.as_str())
    }

    pub fn GetThemeHeight(&self, style_index: usize, ar_index: usize) -> Option<f64> {
        self.styles
            .get(style_index)
            .and_then(|s| s.theme_ars.get(ar_index))
            .map(|ar| ar.height)
    }

    pub fn IsExistingThemeName(&self, name: &str) -> bool {
        self.name_to_packed_index.contains_key(name)
    }

    pub fn GetThemeStyleIndex(&self, name: &str) -> Option<usize> {
        self.name_to_packed_index.get(name).map(|&(s, _)| s)
    }

    pub fn GetThemeAspectRatioIndex(&self, name: &str) -> Option<usize> {
        self.name_to_packed_index.get(name).map(|&(_, a)| a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_to_aspect_ratio_string() {
        assert_eq!(HeightToAspectRatioString(1.0), "1:1");
        assert_eq!(HeightToAspectRatioString(0.5), "2:1");
        assert_eq!(HeightToAspectRatioString(2.0), "1:2");
    }

    #[test]
    fn empty_catalog() {
        let names = emFileManThemeNames::from_themes(&[]);
        assert_eq!(names.GetThemeStyleCount(), 0);
        assert!(!names.IsExistingThemeName("anything"));
        assert!(names.GetDefaultThemeName().is_empty());
    }

    #[test]
    fn single_theme() {
        let names = emFileManThemeNames::from_themes(&[("Glass1", "Glass", "", 1.5)]);
        assert_eq!(names.GetThemeStyleCount(), 1);
        assert!(names.IsExistingThemeName("Glass1"));
        assert_eq!(names.GetThemeStyleIndex("Glass1"), Some(0));
        assert_eq!(names.GetThemeAspectRatioIndex("Glass1"), Some(0));
        assert_eq!(names.GetThemeName(0, 0), Some("Glass1".to_string()));
    }

    #[test]
    fn multiple_aspect_ratios_same_style() {
        let names = emFileManThemeNames::from_themes(&[
            ("Glass1", "Glass", "", 1.0),
            ("Glass2", "Glass", "", 2.0),
        ]);
        assert_eq!(names.GetThemeStyleCount(), 1);
        assert_eq!(names.GetThemeAspectRatioCount(0), 2);
        // Sorted by height: Glass1 (1.0) before Glass2 (2.0)
        assert_eq!(names.GetThemeName(0, 0), Some("Glass1".to_string()));
        assert_eq!(names.GetThemeName(0, 1), Some("Glass2".to_string()));
    }

    #[test]
    fn default_theme_name_prefers_glass1() {
        let names = emFileManThemeNames::from_themes(&[
            ("Glass1", "Glass", "", 1.0),
            ("Other1", "Other", "", 1.0),
        ]);
        assert_eq!(names.GetDefaultThemeName(), "Glass1");
    }

    #[test]
    fn default_theme_falls_back_to_first() {
        let names = emFileManThemeNames::from_themes(&[("Metal1", "Metal", "", 1.0)]);
        assert_eq!(names.GetDefaultThemeName(), "Metal1");
    }

    #[test]
    fn style_display_name() {
        let names =
            emFileManThemeNames::from_themes(&[("Glass1", "Glass Display", "icon.png", 1.0)]);
        assert_eq!(names.GetThemeStyleDisplayName(0), Some("Glass Display"));
        assert_eq!(names.GetThemeStyleDisplayIcon(0), Some("icon.png"));
    }
}
