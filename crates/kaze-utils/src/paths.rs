//! Platform-specific path resolution, isolated here so the rest of the
//! codebase never hardcodes `~/.config` or thinks about XDG directly.
//!
//! This is also the seam where Windows support (§13 of the architecture
//! doc) plugs in later: `directories::ProjectDirs` already resolves to
//! `%APPDATA%\Kaze\...` on Windows for free, so nothing here should need
//! to change when that port happens.

use crate::error::{KazeError, Result};
use directories::ProjectDirs;
use std::path::{Path, PathBuf};

pub struct KazePaths {
    dirs: ProjectDirs,
}

impl KazePaths {
    /// Resolve platform directories for the app. Fails only if the OS
    /// gives us literally nothing to work with (e.g. no HOME on Unix).
    pub fn resolve() -> Result<Self> {
        let dirs = ProjectDirs::from("org", "kaze", "Kaze").ok_or(KazeError::NoPlatformDirs)?;
        Ok(Self { dirs })
    }

    /// `$XDG_CONFIG_HOME/kaze/` — settings.toml lives here.
    pub fn config_dir(&self) -> &Path {
        self.dirs.config_dir()
    }

    /// `$XDG_DATA_HOME/kaze/` — history.db, bookmarks.db, session.json.
    pub fn data_dir(&self) -> &Path {
        self.dirs.data_dir()
    }

    /// `$XDG_CACHE_HOME/kaze/` — adblock compiled filter lists, favicons.
    pub fn cache_dir(&self) -> &Path {
        self.dirs.cache_dir()
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config_dir().join("settings.toml")
    }

    pub fn history_db(&self) -> PathBuf {
        self.data_dir().join("history.sqlite")
    }

    pub fn bookmarks_db(&self) -> PathBuf {
        self.data_dir().join("bookmarks.sqlite")
    }

    pub fn downloads_db(&self) -> PathBuf {
        self.data_dir().join("downloads.sqlite")
    }

    pub fn session_file(&self) -> PathBuf {
        self.data_dir().join("session.json")
    }

    pub fn adblock_cache_dir(&self) -> PathBuf {
        self.cache_dir().join("adblock")
    }

    /// Ensure config/data/cache dirs exist. Call once at startup.
    pub fn ensure_dirs(&self) -> Result<()> {
        for dir in [self.config_dir(), self.data_dir(), self.cache_dir()] {
            std::fs::create_dir_all(dir).map_err(|e| KazeError::io(dir, e))?;
        }
        std::fs::create_dir_all(self.adblock_cache_dir())
            .map_err(|e| KazeError::io(self.adblock_cache_dir(), e))?;
        Ok(())
    }
}
