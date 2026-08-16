//! The settings schema. This struct is the single source of truth for
//! everything a user can configure — the settings UI, `kaze-theme`, and
//! disk persistence all read/write through this same type, so there is no
//! separate "UI model" that can drift out of sync with what's saved.
//!
//! Every field has a sane default (`Default` is derived/implemented for
//! every sub-struct) so that adding a new field never breaks loading an
//! older user's `settings.toml` — missing keys just fall back silently.

use serde::{Deserialize, Serialize};

/// Bumped whenever the schema changes in a way that needs a migration.
/// See `migrations.rs`.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct KazeSettings {
    pub schema_version: u32,
    pub theme: ThemeSettings,
    pub privacy: PrivacySettings,
    pub tabs: TabSettings,
    pub general: GeneralSettings,
}

impl Default for KazeSettings {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            theme: ThemeSettings::default(),
            privacy: PrivacySettings::default(),
            tabs: TabSettings::default(),
            general: GeneralSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ColorScheme {
    Light,
    Dark,
    /// Follow the desktop's `Adw::StyleManager` dark-mode setting.
    System,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::System
    }
}

/// Design tokens consumed by `kaze-theme`. Kept as plain, serializable
/// data (no GTK types) so it can later double as the format for
/// user-installable custom theme files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ThemeSettings {
    /// Hex color, e.g. "#7c6cf0". Validated on load; falls back to the
    /// default accent if malformed rather than failing to start.
    pub accent_color: String,
    pub corner_radius_px: f32,
    /// 0.0 (no blur) .. 1.0 (maximum blur the platform can do).
    pub blur_amount: f32,
    pub sidebar_width_px: u32,
    pub color_scheme: ColorScheme,
    pub animations_enabled: bool,
    pub compact_mode: bool,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            accent_color: "#7c6cf0".to_string(),
            corner_radius_px: 12.0,
            blur_amount: 0.6,
            sidebar_width_px: 240,
            color_scheme: ColorScheme::System,
            animations_enabled: true,
            compact_mode: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PrivacySettings {
    pub adblock_enabled: bool,
    pub tracker_blocking_enabled: bool,
    pub https_upgrade_enabled: bool,
    pub block_third_party_cookies: bool,
    pub adblock_list_update_interval_hours: u32,
    /// Per-origin adblock exceptions (host -> allowed). Kept simple for
    /// v0.1; a dedicated store can replace this if it grows large.
    pub adblock_allowlist: Vec<String>,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            adblock_enabled: true,
            tracker_blocking_enabled: true,
            https_upgrade_enabled: true,
            block_third_party_cookies: true,
            adblock_list_update_interval_hours: 24,
            adblock_allowlist: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct TabSettings {
    /// Kaze ships vertical tabs by default (Zen/Arc-style); this exists
    /// as a setting rather than a hardcoded layout because "highly
    /// customizable" is a stated goal, not because horizontal tabs are
    /// a launch priority.
    pub vertical_tabs: bool,
    pub suspend_inactive_tabs: bool,
    pub suspend_after_minutes: u32,
    pub open_new_tab_next_to_current: bool,
    pub confirm_before_closing_multiple_tabs: bool,
}

impl Default for TabSettings {
    fn default() -> Self {
        Self {
            vertical_tabs: true,
            suspend_inactive_tabs: true,
            suspend_after_minutes: 15,
            open_new_tab_next_to_current: true,
            confirm_before_closing_multiple_tabs: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GeneralSettings {
    pub homepage: String,
    pub search_engine: SearchEngine,
    pub restore_session_on_startup: bool,
    pub default_download_dir: Option<String>,
    pub ask_where_to_save_downloads: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            // A self-contained `data:` URL rather than a custom
            // `about:newtab` scheme — WebKitGTK doesn't recognize
            // app-defined `about:` pages without registering a custom
            // URI scheme handler, which v0.1 doesn't do yet (discovered
            // by actually running the browser: WebKit showed its own
            // "The URL can't be shown" error page for `about:newtab`).
            // A real new-tab page (with a proper URI scheme, quick
            // links, etc.) is a documented follow-up, not a silent gap.
            homepage: "data:text/html,<html><head><title>New Tab</title></head><body style='font-family:sans-serif;display:flex;align-items:center;justify-content:center;height:100vh;margin:0;color:%23888'><h1>Kaze</h1></body></html>".to_string(),
            search_engine: SearchEngine::default(),
            restore_session_on_startup: true,
            default_download_dir: None,
            ask_where_to_save_downloads: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SearchEngine {
    DuckDuckGo,
    Startpage,
    Brave,
    Custom(String), // URL template with `%s` placeholder
}

impl Default for SearchEngine {
    fn default() -> Self {
        // Privacy-respecting default, in keeping with "privacy by default".
        Self::DuckDuckGo
    }
}

impl SearchEngine {
    pub fn query_url(&self, query: &str) -> String {
        let encoded = urlencoding_minimal(query);
        match self {
            Self::DuckDuckGo => format!("https://duckduckgo.com/?q={encoded}"),
            Self::Startpage => format!("https://www.startpage.com/sp/search?query={encoded}"),
            Self::Brave => format!("https://search.brave.com/search?q={encoded}"),
            Self::Custom(template) => template.replace("%s", &encoded),
        }
    }
}

/// Minimal percent-encoding so this crate doesn't need a full `url` or
/// `urlencoding` dependency for one call site. Covers the common case
/// (spaces and reserved characters in search queries).
fn urlencoding_minimal(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}
