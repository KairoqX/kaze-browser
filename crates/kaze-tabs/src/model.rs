//! The tab data model. Deliberately GTK-free — see §7 of the architecture
//! doc: the sidebar's `GtkListView` binds to [`TabStore`] via a
//! `gio::ListModel` adapter written in `kaze-ui`, but the model itself
//! (this file) has zero knowledge that GTK exists, which is what makes
//! it possible to unit-test "open/close/reorder N tabs" in milliseconds.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(pub Uuid);

impl TabId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TabId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TabId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Distinguishes normal tabs from incognito ones. This is what
/// `kaze-engine-webkit` uses to decide which `WebContext` (and therefore
/// which cookie jar / cache / disk persistence) a tab's `WebView` gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProfileId {
    Normal,
    /// Ephemeral — never written to `kaze-history`, `kaze-session`, or
    /// the adblock allowlist. All ephemeral incognito tabs in a single
    /// window share one isolated in-memory profile; each incognito
    /// *window* gets its own.
    Incognito(Uuid),
}

impl ProfileId {
    pub fn is_incognito(&self) -> bool {
        matches!(self, Self::Incognito(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tab {
    pub id: TabId,
    pub url: String,
    pub title: String,
    /// Raw favicon bytes, if loaded. Kept optional/lazy — most tab list
    /// operations don't need it, and session snapshots may choose to
    /// omit it to keep the session file small.
    pub favicon: Option<Vec<u8>>,
    pub is_loading: bool,
    pub is_pinned: bool,
    pub is_muted: bool,
    pub profile: ProfileId,
    /// The tab this one was opened from (e.g. via ctrl-click / target=_blank),
    /// used for "close tab and its orphaned children" behavior and as the
    /// seed for future tab-tree/grouping UI.
    pub parent: Option<TabId>,
    /// Suspended tabs have no live `WebView` backing them — see §6
    /// (rendering pipeline) and the `suspend_inactive_tabs` setting.
    /// Reactivating a suspended tab reloads `url` fresh.
    pub is_suspended: bool,
}

impl Tab {
    pub fn new(url: impl Into<String>, profile: ProfileId) -> Self {
        let url = url.into();
        Self {
            id: TabId::new(),
            title: url.clone(),
            url,
            favicon: None,
            is_loading: true,
            is_pinned: false,
            is_muted: false,
            profile,
            parent: None,
            is_suspended: false,
        }
    }

    pub fn child_of(url: impl Into<String>, profile: ProfileId, parent: TabId) -> Self {
        let mut tab = Self::new(url, profile);
        tab.parent = Some(parent);
        tab
    }
}
