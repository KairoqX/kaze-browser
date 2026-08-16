//! [`TabStore`] — owns the ordered list of tabs and the currently active
//! tab, and is the single source of truth the sidebar, toolbar, and
//! window title all read from (see "State Management", §4 of the
//! architecture doc). Mutations go through methods on this type, never
//! by widgets touching `Tab` fields directly.
//!
//! Events are QUEUED rather than dispatched synchronously (see
//! [`TabStore::take_events`]). Earlier versions of this store called
//! subscriber closures immediately from inside `emit`, which is a real
//! footgun for exactly the kind of caller this store expects: a UI layer
//! holding the store behind `Rc<RefCell<TabStore>>>`. A caller doing
//! `tabs.borrow_mut().open(...)` would trigger a synchronous callback
//! that itself tried `tabs.borrow()` to read current state — a reentrant
//! borrow that panics at runtime with "already mutably borrowed". This
//! surfaced immediately the first time the app was actually run (not
//! just compiled), which is exactly the kind of bug `cargo build` alone
//! can't catch. Queuing events and draining them after the mutating call
//! returns (and the borrow is released) sidesteps the reentrancy
//! entirely while keeping the same "mutate, then react" flow from the
//! architecture doc.
use crate::model::{ProfileId, Tab, TabId};

#[derive(Debug, Clone)]
pub enum TabEvent {
    Created { id: TabId, index: usize },
    Closed { id: TabId },
    Activated { id: TabId },
    Updated { id: TabId },
    Reordered,
}

#[derive(Default)]
pub struct TabStore {
    tabs: Vec<Tab>,
    active: Option<TabId>,
    pending_events: Vec<TabEvent>,
}

impl TabStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn emit(&mut self, event: TabEvent) {
        self.pending_events.push(event);
    }

    /// Drain and return every event queued since the last call to this
    /// method. Callers should invoke this immediately after a mutating
    /// call *outside* of any borrow they need to hold for the mutation
    /// itself — e.g. `{ tabs.borrow_mut().open(...) }; let events =
    /// tabs.borrow_mut().take_events();` as two separate short borrows,
    /// not one long one, so reacting to the events (which typically
    /// means reading `tabs.borrow()` again) can't collide with a
    /// still-open mutable borrow.
    pub fn take_events(&mut self) -> Vec<TabEvent> {
        std::mem::take(&mut self.pending_events)
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn active_id(&self) -> Option<TabId> {
        self.active
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.active.and_then(|id| self.get(id))
    }

    pub fn get(&self, id: TabId) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == id)
    }

    fn index_of(&self, id: TabId) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    /// Open a new tab. If `next_to_active` is true and there is an active
    /// tab, the new tab is inserted immediately after it (matching the
    /// `open_new_tab_next_to_current` setting); otherwise it's appended
    /// to the end. Returns the new tab's id.
    pub fn open(&mut self, url: impl Into<String>, profile: ProfileId, next_to_active: bool) -> TabId {
        let tab = Tab::new(url, profile);
        let id = tab.id;

        let index = if next_to_active {
            self.active
                .and_then(|active_id| self.index_of(active_id))
                .map(|i| i + 1)
                .unwrap_or(self.tabs.len())
        } else {
            self.tabs.len()
        };

        self.tabs.insert(index, tab);
        self.emit(TabEvent::Created { id, index });
        id
    }

    /// Opens a tab as a child of `parent` (e.g. a link opened in a new
    /// tab), inserted immediately after its parent.
    pub fn open_child(&mut self, url: impl Into<String>, profile: ProfileId, parent: TabId) -> TabId {
        let tab = Tab::child_of(url, profile, parent);
        let id = tab.id;
        let index = self.index_of(parent).map(|i| i + 1).unwrap_or(self.tabs.len());
        self.tabs.insert(index, tab);
        self.emit(TabEvent::Created { id, index });
        id
    }

    /// Closes a tab. If it was active, activates a sensible neighbor
    /// (prefers the tab to the right, falls back to the left, then to
    /// nothing if the store is now empty) — this mirrors the behavior
    /// users expect from Firefox/Chrome-style tab strips.
    pub fn close(&mut self, id: TabId) {
        let Some(index) = self.index_of(id) else {
            return;
        };

        self.tabs.remove(index);
        self.emit(TabEvent::Closed { id });

        if self.active == Some(id) {
            let next_active = self
                .tabs
                .get(index) // the tab that slid into this index (was to the right)
                .or_else(|| index.checked_sub(1).and_then(|i| self.tabs.get(i)))
                .map(|t| t.id);
            self.active = next_active;
            if let Some(new_active) = next_active {
                self.emit(TabEvent::Activated { id: new_active });
            }
        }
    }

    pub fn activate(&mut self, id: TabId) {
        if self.index_of(id).is_none() || self.active == Some(id) {
            return;
        }
        self.active = Some(id);
        self.emit(TabEvent::Activated { id });
    }

    /// Mutate a tab's fields (title/url/loading state/etc.) via `f`,
    /// emitting `Updated` afterward. This is how `kaze-engine` events
    /// (title changed, favicon changed, load state changed) get applied.
    pub fn update(&mut self, id: TabId, f: impl FnOnce(&mut Tab)) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            f(tab);
            self.emit(TabEvent::Updated { id });
        }
    }

    /// Move the tab at `from` to `to` (both are positions, not ids),
    /// used by sidebar drag-and-drop reordering.
    pub fn reorder(&mut self, from: usize, to: usize) {
        if from == to || from >= self.tabs.len() || to >= self.tabs.len() {
            return;
        }
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        self.emit(TabEvent::Reordered);
    }

    /// All tabs belonging to a given profile, e.g. for closing an
    /// incognito window's tabs together.
    pub fn tabs_in_profile(&self, profile: ProfileId) -> impl Iterator<Item = &Tab> {
        self.tabs.iter().filter(move |t| t.profile == profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn open_appends_and_activates_manually() {
        let mut store = TabStore::new();
        let id1 = store.open("https://a.example", ProfileId::Normal, false);
        let id2 = store.open("https://b.example", ProfileId::Normal, false);

        assert_eq!(store.len(), 2);
        assert_eq!(store.tabs()[0].id, id1);
        assert_eq!(store.tabs()[1].id, id2);
        assert_eq!(store.active_id(), None); // opening doesn't auto-activate

        store.activate(id2);
        assert_eq!(store.active_id(), Some(id2));
    }

    #[test]
    fn open_next_to_active_inserts_after_active_not_at_end() {
        let mut store = TabStore::new();
        let id1 = store.open("https://a.example", ProfileId::Normal, false);
        let _id2 = store.open("https://b.example", ProfileId::Normal, false);
        store.activate(id1);

        let id3 = store.open("https://c.example", ProfileId::Normal, true);
        assert_eq!(store.tabs()[1].id, id3); // inserted right after id1, not appended
    }

    #[test]
    fn closing_active_tab_activates_right_neighbor() {
        let mut store = TabStore::new();
        let id1 = store.open("https://a.example", ProfileId::Normal, false);
        let id2 = store.open("https://b.example", ProfileId::Normal, false);
        let id3 = store.open("https://c.example", ProfileId::Normal, false);
        store.activate(id2);

        store.close(id2);
        assert_eq!(store.active_id(), Some(id3));
        assert_eq!(store.len(), 2);
        let _ = id1;
    }

    #[test]
    fn closing_last_tab_falls_back_to_left_neighbor() {
        let mut store = TabStore::new();
        let id1 = store.open("https://a.example", ProfileId::Normal, false);
        let id2 = store.open("https://b.example", ProfileId::Normal, false);
        store.activate(id2);

        store.close(id2);
        assert_eq!(store.active_id(), Some(id1));
    }

    #[test]
    fn update_emits_updated_event() {
        let mut store = TabStore::new();
        let id = store.open("https://a.example", ProfileId::Normal, false);
        store.take_events(); // discard the Created event from open()

        store.update(id, |tab| tab.title = "New Title".to_string());
        assert_eq!(store.get(id).unwrap().title, "New Title");

        let events = store.take_events();
        assert!(events
            .iter()
            .any(|e| matches!(e, TabEvent::Updated { id: eid } if *eid == id)));
    }

    #[test]
    fn reorder_moves_tab_between_positions() {
        let mut store = TabStore::new();
        let id1 = store.open("https://a.example", ProfileId::Normal, false);
        let id2 = store.open("https://b.example", ProfileId::Normal, false);
        let id3 = store.open("https://c.example", ProfileId::Normal, false);

        store.reorder(0, 2); // move id1 to the end
        assert_eq!(store.tabs()[0].id, id2);
        assert_eq!(store.tabs()[1].id, id3);
        assert_eq!(store.tabs()[2].id, id1);
    }

    #[test]
    fn incognito_tabs_are_isolated_by_profile() {
        let mut store = TabStore::new();
        let profile_a = ProfileId::Incognito(uuid::Uuid::new_v4());
        let profile_b = ProfileId::Incognito(uuid::Uuid::new_v4());

        store.open("https://a.example", profile_a, false);
        store.open("https://b.example", profile_b, false);
        store.open("https://c.example", ProfileId::Normal, false);

        assert_eq!(store.tabs_in_profile(profile_a).count(), 1);
        assert_eq!(store.tabs_in_profile(ProfileId::Normal).count(), 1);
    }

    /// Regression test for the exact reentrancy bug found by actually
    /// running `kaze-app`: a caller holding `Rc<RefCell<TabStore>>>`
    /// must be able to mutate the store, then read it again to react to
    /// the resulting events, without the mutating call itself trying to
    /// re-borrow. This models `BrowserWindow::open_tab`'s real usage
    /// pattern in `kaze-ui`.
    #[test]
    fn events_can_be_drained_after_releasing_the_mutable_borrow() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let store = Rc::new(RefCell::new(TabStore::new()));

        // Mutate via a short-lived borrow_mut, exactly like
        // `open_tab` does.
        let id = store.borrow_mut().open("https://a.example", ProfileId::Normal, false);

        // Drain events via a SEPARATE short borrow_mut...
        let events = store.borrow_mut().take_events();

        // ...which means it's safe to read the store again here, the
        // way a UI event handler would (e.g. to sync a sidebar).
        let snapshot = store.borrow(); // must not panic
        assert_eq!(snapshot.tabs().len(), 1);
        assert_eq!(snapshot.tabs()[0].id, id);
        assert!(events
            .iter()
            .any(|e| matches!(e, TabEvent::Created { id: eid, .. } if *eid == id)));
    }
}
