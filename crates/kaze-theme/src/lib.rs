//! `kaze-theme` — the theme engine described in §10 of the architecture
//! doc: turns persisted [`kaze_settings::ThemeSettings`] into a GTK CSS
//! string, with system light/dark resolution handled explicitly rather
//! than implicitly, so this crate stays GTK-free and unit-testable.
//!
//! `kaze-ui` is expected to:
//! 1. Subscribe to `SettingsStore` changes.
//! 2. Call [`tokens::resolve`] with the current settings + current
//!    `Adw::StyleManager::is_dark()`.
//! 3. Call [`css::generate_css`] and load the result into a
//!    `gtk4::CssProvider`.
//! 4. Re-run 2–3 whenever settings change *or* the system theme flips.

pub mod css;
pub mod tokens;

pub use css::generate_css;
pub use tokens::{resolve, ResolvedScheme, ResolvedTheme, Rgb};
