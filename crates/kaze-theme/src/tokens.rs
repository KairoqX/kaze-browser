//! Resolves the user's [`ThemeSettings`] (persisted config) into concrete
//! [`ResolvedTheme`] values ready for CSS generation.
//!
//! The distinction matters: `ThemeSettings::color_scheme` can be `System`,
//! which isn't a real color scheme by itself — it has to be resolved
//! against whatever `Adw::StyleManager` currently reports. Keeping that
//! resolution step here (rather than baking "System" handling into the
//! CSS generator) keeps `kaze-theme` GTK-free and unit-testable: callers
//! just pass in `is_dark: bool` from wherever they got it.

use kaze_settings::{ColorScheme, ThemeSettings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedScheme {
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTheme {
    pub accent: Rgb,
    pub corner_radius_px: f32,
    pub blur_amount: f32,
    pub sidebar_width_px: u32,
    pub scheme: ResolvedScheme,
    pub animations_enabled: bool,
    pub compact_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn to_css(self) -> String {
        format!("rgb({}, {}, {})", self.r, self.g, self.b)
    }

    /// Parses a `#rrggbb` hex string. Falls back to Kaze's default accent
    /// (`#7c6cf0`) on malformed input rather than erroring — a bad hex
    /// value in a hand-edited config should never stop the browser from
    /// starting.
    pub fn from_hex(hex: &str) -> Self {
        let default = Self {
            r: 0x7c,
            g: 0x6c,
            b: 0xf0,
        };
        let hex = hex.trim().trim_start_matches('#');
        if hex.len() != 6 {
            return default;
        }
        let parse = |s: &str| u8::from_str_radix(s, 16).ok();
        match (parse(&hex[0..2]), parse(&hex[2..4]), parse(&hex[4..6])) {
            (Some(r), Some(g), Some(b)) => Self { r, g, b },
            _ => default,
        }
    }
}

/// Resolve persisted [`ThemeSettings`] into concrete values, given whether
/// the desktop is currently in dark mode (irrelevant unless
/// `color_scheme == System`).
pub fn resolve(settings: &ThemeSettings, system_is_dark: bool) -> ResolvedTheme {
    let scheme = match settings.color_scheme {
        ColorScheme::Light => ResolvedScheme::Light,
        ColorScheme::Dark => ResolvedScheme::Dark,
        ColorScheme::System => {
            if system_is_dark {
                ResolvedScheme::Dark
            } else {
                ResolvedScheme::Light
            }
        }
    };

    ResolvedTheme {
        accent: Rgb::from_hex(&settings.accent_color),
        corner_radius_px: settings.corner_radius_px.clamp(0.0, 32.0),
        blur_amount: settings.blur_amount.clamp(0.0, 1.0),
        sidebar_width_px: settings.sidebar_width_px.clamp(160, 480),
        scheme,
        animations_enabled: settings.animations_enabled,
        compact_mode: settings.compact_mode,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_hex() {
        let c = Rgb::from_hex("#ff8800");
        assert_eq!(c, Rgb { r: 0xff, g: 0x88, b: 0x00 });
    }

    #[test]
    fn falls_back_on_bad_hex() {
        let c = Rgb::from_hex("not-a-color");
        assert_eq!(c, Rgb { r: 0x7c, g: 0x6c, b: 0xf0 });
    }

    #[test]
    fn clamps_out_of_range_values() {
        let mut settings = ThemeSettings::default();
        settings.corner_radius_px = 999.0;
        settings.sidebar_width_px = 10;
        let resolved = resolve(&settings, false);
        assert_eq!(resolved.corner_radius_px, 32.0);
        assert_eq!(resolved.sidebar_width_px, 160);
    }

    #[test]
    fn resolves_system_scheme_against_dark_flag() {
        let settings = ThemeSettings {
            color_scheme: ColorScheme::System,
            ..ThemeSettings::default()
        };
        assert_eq!(resolve(&settings, true).scheme, ResolvedScheme::Dark);
        assert_eq!(resolve(&settings, false).scheme, ResolvedScheme::Light);
    }
}
