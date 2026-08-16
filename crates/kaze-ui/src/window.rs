//! The main browser window: wires [`TabStore`], a [`RenderEngine`], the
//! sidebar, and the toolbar together following the unidirectional data
//! flow described in architecture doc §4.
//!
//! `BrowserWindow` is the one place in `kaze-ui` that's allowed to know
//! about all three of `kaze-tabs`, `kaze-engine`, and the widgets below
//! it — everything downstream of it should stay dumb (render state,
//! emit intents) per the architecture's UI principle.

use crate::sidebar::Sidebar;
use crate::toolbar::Toolbar;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Orientation, Paned};
use kaze_engine::{EngineEvent, EngineView, RenderEngine, ViewConfig};
use kaze_tabs::{ProfileId, TabEvent, TabId, TabStore};
use libadwaita::prelude::*;
use libadwaita::ApplicationWindow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct BrowserWindow {
    pub window: ApplicationWindow,
    tabs: Rc<RefCell<TabStore>>,
    engine: Rc<dyn RenderEngine>,
    views: Rc<RefCell<HashMap<TabId, Box<dyn EngineView>>>>,
    content_area: GtkBox,
    sidebar: Rc<Sidebar>,
    toolbar: Rc<Toolbar>,
    homepage: String,
}

impl BrowserWindow {
    pub fn new(
        app: &libadwaita::Application,
        engine: Rc<dyn RenderEngine>,
        homepage: String,
        sidebar_width: u32,
    ) -> Rc<Self> {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Kaze")
            .default_width(1280)
            .default_height(800)
            .build();

        let sidebar = Sidebar::new();
        sidebar.root.set_size_request(sidebar_width as i32, -1);

        let toolbar = Toolbar::new();

        let content_area = GtkBox::new(Orientation::Vertical, 0);
        content_area.append(&toolbar.root);

        let paned = Paned::builder()
            .orientation(Orientation::Horizontal)
            .start_child(&sidebar.root)
            .end_child(&content_area)
            .position(sidebar_width as i32)
            .build();

        window.set_content(Some(&paned));

        let browser = Rc::new(Self {
            window,
            tabs: Rc::new(RefCell::new(TabStore::new())),
            engine,
            views: Rc::new(RefCell::new(HashMap::new())),
            content_area,
            sidebar,
            toolbar,
            homepage,
        });

        Self::wire_sidebar(&browser);
        Self::wire_toolbar(&browser);

        // Every browser window starts with one tab, matching the
        // "New tab page" v0.1 feature — an empty window with zero tabs
        // isn't a state the rest of the UI needs to handle.
        let homepage = browser.homepage.clone();
        browser.open_tab(homepage, ProfileId::Normal, false);

        browser
    }

    /// Drain whatever `TabEvent`s have queued up since the last drain
    /// and react to them: sync the sidebar, show/hide tab content,
    /// tear down closed views, refresh the toolbar. Callers invoke this
    /// immediately after any `TabStore` mutation, and specifically
    /// *after* the `borrow_mut()` used for that mutation has already
    /// gone out of scope — see the module doc on `kaze_tabs::TabStore`
    /// for why that ordering matters (this is the fix for a reentrant
    /// `RefCell` panic found by actually running the app).
    fn process_pending_tab_events(self: &Rc<Self>) {
        let events = self.tabs.borrow_mut().take_events();
        if events.is_empty() {
            return;
        }

        // Keep the sidebar in sync with whatever just happened. v0.1
        // takes the simple "rebuild the whole row list" path documented
        // in sidebar.rs rather than fine-grained per-event patching.
        self.sidebar.sync(&self.tabs.borrow());

        let mut should_refresh_toolbar = false;
        for event in &events {
            match *event {
                TabEvent::Activated { id } => {
                    self.show_tab_content(id);
                    should_refresh_toolbar = true;
                }
                TabEvent::Closed { id } => self.teardown_view(id),
                TabEvent::Updated { id } => {
                    if self.tabs.borrow().active_id() == Some(id) {
                        should_refresh_toolbar = true;
                    }
                }
                TabEvent::Created { .. } | TabEvent::Reordered => {}
            }
        }

        if should_refresh_toolbar {
            self.refresh_toolbar_for_active();
        }
    }

    fn refresh_toolbar_for_active(self: &Rc<Self>) {
        let tabs = self.tabs.borrow();
        if let Some(tab) = tabs.active_tab() {
            self.toolbar.set_address(&tab.url);
            if let Some(view) = self.views.borrow().get(&tab.id) {
                self.toolbar
                    .set_nav_state(view.can_go_back(), view.can_go_forward());
            }
        }
    }

    fn wire_sidebar(browser: &Rc<Self>) {
        let b = browser.clone();
        browser.sidebar.on_activate(move |id| {
            b.tabs.borrow_mut().activate(id);
            b.process_pending_tab_events();
        });

        let b = browser.clone();
        browser.sidebar.on_close(move |id| {
            b.tabs.borrow_mut().close(id);
            b.process_pending_tab_events();
        });

        let b = browser.clone();
        browser.sidebar.on_new_tab(move || {
            let homepage = b.homepage.clone();
            b.open_tab(homepage, ProfileId::Normal, true);
        });
    }

    fn wire_toolbar(browser: &Rc<Self>) {
        let b = browser.clone();
        browser.toolbar.on_navigate(move |input| {
            let url = normalize_input_to_url(&input);
            let url = crate::newtab::resolve(&url); 
            // Read active_id as its own statement so the `Ref` from
            // `borrow()` is dropped immediately — NOT held across the
            // `view.load_url()` call below. WebKit fires `notify::uri`
            // synchronously from inside `load_uri()`, which re-enters this
            // same `tabs` RefCell via `borrow_mut()`. Holding the borrow
            // across the call caused a real "RefCell already borrowed"
            // panic on Enter in the address bar.
            let active = b.tabs.borrow().active_id();
            if let Some(active) = active {
                if let Some(view) = b.views.borrow().get(&active) {
                    view.load_url(&url);
                }
            }
        });

        let b = browser.clone();
        browser.toolbar.on_back(move || {
            let active = b.tabs.borrow().active_id();
            if let Some(active) = active {
                if let Some(view) = b.views.borrow().get(&active) {
                    view.go_back();
                }
            }
        });

        let b = browser.clone();
        browser.toolbar.on_forward(move || {
            let active = b.tabs.borrow().active_id();
            if let Some(active) = active {
                if let Some(view) = b.views.borrow().get(&active) {
                    view.go_forward();
                }
            }
        });

        let b = browser.clone();
        browser.toolbar.on_reload(move || {
            let active = b.tabs.borrow().active_id();
            if let Some(active) = active {
                if let Some(view) = b.views.borrow().get(&active) {
                    view.reload();
                }
            }
        });
    }

    /// Opens a new tab, creates its backing engine view, wires the
    /// view's events back into `TabStore::update` (translating
    /// `kaze-engine`'s vocabulary into tab-model mutations, per
    /// architecture doc §7), and activates it.
    pub fn open_tab(self: &Rc<Self>, url: String, profile: ProfileId, next_to_active: bool) -> TabId {
        let id = self.tabs.borrow_mut().open(url.clone(), profile, next_to_active);
        let resolved_url = crate::newtab::resolve(&url);
        let view = self.engine.create_view(ViewConfig {
            profile,
            initial_url: resolved_url,
            enable_javascript: true,
            user_agent: None,
        });

        self.content_area.append(view.widget());
        view.widget().set_visible(false);
        view.widget().set_vexpand(true);

        let tabs = self.tabs.clone();
        let browser = self.clone();
        view.on_event(Box::new(move |event| {
            apply_engine_event_to_tab(&tabs, id, event);
            browser.process_pending_tab_events();
        }));

        self.views.borrow_mut().insert(id, view);
        self.tabs.borrow_mut().activate(id);
        self.process_pending_tab_events();

        id
    }

    fn show_tab_content(&self, active_id: TabId) {
        let views = self.views.borrow();
        for (id, view) in views.iter() {
            view.widget().set_visible(*id == active_id);
        }
    }

    fn teardown_view(&self, id: TabId) {
        if let Some(view) = self.views.borrow_mut().remove(&id) {
            self.content_area.remove(view.widget());
        }
    }
}

fn apply_engine_event_to_tab(tabs: &Rc<RefCell<TabStore>>, id: TabId, event: EngineEvent) {
    tabs.borrow_mut().update(id, |tab| match event {
        EngineEvent::LoadStarted => tab.is_loading = true,
        EngineEvent::LoadFinished => tab.is_loading = false,
        EngineEvent::LoadFailed { .. } => tab.is_loading = false,
        EngineEvent::LoadProgress(_) => {}
        EngineEvent::TitleChanged(title) => tab.title = title,
        EngineEvent::UrlChanged(url) => tab.url = url,
        EngineEvent::FaviconChanged(bytes) => tab.favicon = Some(bytes),
        EngineEvent::NewWindowRequested { .. } => {}
        EngineEvent::DownloadRequested { .. } => {}
        EngineEvent::InsecureNavigationAttempted { .. } => {}
    });
}

/// Turns whatever the user typed in the address bar into a URL: if it
/// looks like a URL (has a scheme, or looks like `host.tld`), use it
/// as-is (adding `https://` if bare); otherwise treat it as a search
/// query. Kept deliberately simple for v0.1 — no punycode/IDN handling,
/// no `about:` scheme special-casing beyond what WebKit does natively.
fn normalize_input_to_url(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "about:blank".to_string();
    }
    if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("about:")
        || trimmed.starts_with("file://")
    {
        return trimmed.to_string();
    }
    let looks_like_host = trimmed.contains('.') && !trimmed.contains(' ');
    if looks_like_host {
        format!("https://{trimmed}")
    } else {
        kaze_settings::SearchEngine::default().query_url(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_domain_gets_https_scheme() {
        assert_eq!(normalize_input_to_url("example.com"), "https://example.com");
    }

    #[test]
    fn full_url_passes_through() {
        assert_eq!(normalize_input_to_url("https://example.com/x"), "https://example.com/x");
    }

    #[test]
    fn plain_words_become_a_search() {
        assert!(normalize_input_to_url("rust programming language").starts_with("https://duckduckgo.com"));
    }

    #[test]
    fn empty_input_goes_to_blank() {
        assert_eq!(normalize_input_to_url("   "), "about:blank");
    }
}
