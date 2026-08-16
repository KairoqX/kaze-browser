//! `kaze-settings` — the typed, persisted, live-reloadable settings
//! system described in §9 of the architecture doc.
//!
//! No GTK dependency: the settings *UI* (an `AdwPreferencesWindow` bound
//! to this schema) lives in `kaze-ui`, not here, so this crate can be
//! unit-tested headlessly and reused by anything (CLI tools, tests)
//! that needs to read/write Kaze config.

pub mod migrations;
pub mod schema;
pub mod store;

pub use schema::{
    ColorScheme, GeneralSettings, KazeSettings, PrivacySettings, SearchEngine, TabSettings,
    ThemeSettings,
};
pub use store::{ChangeListener, SettingsStore};
