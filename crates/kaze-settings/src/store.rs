//! The runtime settings store: owns the current [`KazeSettings`], persists
//! changes to disk, and notifies subscribers (theme engine, UI) when
//! something changes — this is the concrete implementation of the
//! "Settings System" section of the architecture doc.
//!
//! Threading model: this store is designed to live on the GTK main thread,
//! wrapped in `Rc<RefCell<SettingsStore>>` by `kaze-app`. It is
//! deliberately *not* `Send`/`Sync` — cross-thread settings access should
//! go through a channel into the main thread, not through this type
//! directly, matching the state-management model in the architecture doc.

use crate::migrations::migrate;
use crate::schema::KazeSettings;
use kaze_utils::error::{KazeError, Result};
use std::path::{Path, PathBuf};

/// Callback invoked whenever settings change, with the new value.
pub type ChangeListener = Box<dyn Fn(&KazeSettings)>;

pub struct SettingsStore {
    settings: KazeSettings,
    path: PathBuf,
    listeners: Vec<ChangeListener>,
}

impl SettingsStore {
    /// Load settings from `path`, creating defaults if the file doesn't
    /// exist yet. Never fails just because the file is absent — a fresh
    /// install should start up fine with defaults.
    pub fn load(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let settings = Self::read_from_disk(&path)?;
        Ok(Self {
            settings,
            path,
            listeners: Vec::new(),
        })
    }

    fn read_from_disk(path: &Path) -> Result<KazeSettings> {
        if !path.exists() {
            tracing::info!(?path, "no settings file yet, using defaults");
            return Ok(KazeSettings::default());
        }

        let raw = std::fs::read_to_string(path).map_err(|e| KazeError::io(path, e))?;
        let value: toml::Value = toml::from_str(&raw)?;
        let migrated = migrate(value);
        let settings: KazeSettings = migrated.try_into().map_err(|e: toml::de::Error| {
            tracing::warn!(error = %e, "settings file failed to parse after migration, falling back to defaults");
            e
        }).unwrap_or_default();

        Ok(settings)
    }

    pub fn current(&self) -> &KazeSettings {
        &self.settings
    }

    /// Register a listener that fires on every future change. Does not
    /// fire immediately with the current value — callers that need the
    /// current value should read [`Self::current`] once at setup time.
    pub fn subscribe(&mut self, listener: ChangeListener) {
        self.listeners.push(listener);
    }

    /// Mutate settings via `f`, then persist to disk and notify all
    /// subscribers. This is the *only* way settings should change at
    /// runtime — never mutate `current()`'s result directly (it's `&`,
    /// not `&mut`, specifically to prevent that).
    pub fn update(&mut self, f: impl FnOnce(&mut KazeSettings)) -> Result<()> {
        f(&mut self.settings);
        self.save()?;
        for listener in &self.listeners {
            listener(&self.settings);
        }
        Ok(())
    }

    fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| KazeError::io(parent, e))?;
        }
        let toml_str = toml::to_string_pretty(&self.settings)?;
        std::fs::write(&self.path, toml_str).map_err(|e| KazeError::io(&self.path, e))?;
        tracing::debug!(path = ?self.path, "settings saved");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::ColorScheme;

    #[test]
    fn loads_defaults_when_file_missing() {
        let store = SettingsStore::load("/tmp/kaze-test-does-not-exist/settings.toml").unwrap();
        assert_eq!(store.current().theme.color_scheme, ColorScheme::System);
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = std::env::temp_dir().join(format!("kaze-test-{}", uuid_like()));
        let path = dir.join("settings.toml");

        let mut store = SettingsStore::load(&path).unwrap();
        store
            .update(|s| {
                s.theme.accent_color = "#ff0000".to_string();
                s.theme.corner_radius_px = 20.0;
            })
            .unwrap();

        let reloaded = SettingsStore::load(&path).unwrap();
        assert_eq!(reloaded.current().theme.accent_color, "#ff0000");
        assert_eq!(reloaded.current().theme.corner_radius_px, 20.0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn notifies_listeners_on_update() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let dir = std::env::temp_dir().join(format!("kaze-test-{}", uuid_like()));
        let path = dir.join("settings.toml");

        let mut store = SettingsStore::load(&path).unwrap();
        let seen = Rc::new(RefCell::new(false));
        let seen_clone = seen.clone();
        store.subscribe(Box::new(move |_settings| {
            *seen_clone.borrow_mut() = true;
        }));

        store.update(|s| s.theme.compact_mode = true).unwrap();
        assert!(*seen.borrow());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // Tiny non-random unique-ish suffix so parallel tests don't collide,
    // without pulling in a `rand` dependency just for tests.
    fn uuid_like() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }
}
