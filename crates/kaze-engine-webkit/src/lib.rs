//! `kaze-engine-webkit` — the WebKitGTK 6 implementation of the
//! `kaze-engine` traits. This is the ONLY crate in the workspace that
//! imports `webkit6` directly (see architecture doc §3) — if Kaze ever
//! grows a second backend (e.g. `kaze-engine-webview2` for Windows, per
//! §13), it lives alongside this one and `kaze-ui` never notices the
//! difference.

use gtk4::prelude::*;
use kaze_engine::{EngineEvent, EngineEventCallback, EngineView, RenderEngine, ViewConfig};
use kaze_tabs::ProfileId;
use std::cell::RefCell;
use std::rc::Rc;
use webkit6::prelude::*;
use webkit6::{LoadEvent, WebContext, WebView};

/// WebKitGTK-backed [`RenderEngine`]. Owns one [`WebContext`] per profile
/// kind: a single shared context for normal browsing (persistent cookies
/// / cache on disk), and a fresh ephemeral context created per incognito
/// profile id so incognito windows never share state with each other or
/// with normal browsing — see architecture doc §7 "per-tab process/
/// profile isolation".
pub struct WebkitEngine {
    normal_context: WebContext,
    incognito_contexts: RefCell<std::collections::HashMap<uuid::Uuid, WebContext>>,
}

impl WebkitEngine {
    pub fn new() -> Self {
        Self {
            normal_context: WebContext::default().expect("default WebContext should always be constructible"),
            incognito_contexts: RefCell::new(std::collections::HashMap::new()),
        }
    }

    fn context_for(&self, profile: ProfileId) -> WebContext {
        match profile {
            ProfileId::Normal => self.normal_context.clone(),
            ProfileId::Incognito(id) => {
                let mut contexts = self.incognito_contexts.borrow_mut();
                contexts
                    .entry(id)
                    .or_insert_with(|| {
                        // A fresh, non-persistent WebContext. WebKitGTK's
                        // default WebContext::new() is already
                        // process-local; what makes this "incognito" in
                        // practice is that it's never reused across
                        // windows and its WebsiteDataManager is left at
                        // ephemeral defaults (no explicit disk paths
                        // configured), so cookies/cache die with it.
                        tracing::debug!(profile_id = %id, "creating ephemeral incognito WebContext");
                        WebContext::new()
                    })
                    .clone()
            }
        }
    }

    /// Drop a closed incognito window's context, discarding its
    /// in-memory cookies/cache immediately rather than waiting for
    /// process exit.
    pub fn drop_incognito_profile(&self, id: uuid::Uuid) {
        self.incognito_contexts.borrow_mut().remove(&id);
    }
}

impl Default for WebkitEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderEngine for WebkitEngine {
    fn create_view(&self, config: ViewConfig) -> Box<dyn EngineView> {
        let context = self.context_for(config.profile);

        let web_view = WebView::builder().web_context(&context).build();

        if let Some(settings) = WebViewExt::settings(&web_view) {
            settings.set_enable_javascript(config.enable_javascript);
            if let Some(ua) = &config.user_agent {
                settings.set_user_agent(Some(ua));
            }
        }

        let view = Rc::new(WebkitView {
            web_view,
            listeners: RefCell::new(Vec::new()),
            suspended: RefCell::new(false),
            suspended_url: RefCell::new(None),
        });

        WebkitView::wire_signals(&view);

        if !config.initial_url.is_empty() {
            view.web_view.load_uri(&config.initial_url);
        }

        Box::new(WebkitViewHandle(view))
    }

    fn name(&self) -> String {
        format!("WebKitGTK {}", webkit6::functions::major_version())
    }
}

/// Internal shared state for one tab's WebView. Wrapped in `Rc` because
/// GTK signal closures need their own handle to it (they're `'static`
/// closures registered on the `WebView` itself).
struct WebkitView {
    web_view: WebView,
    listeners: RefCell<Vec<EngineEventCallback>>,
    suspended: RefCell<bool>,
    suspended_url: RefCell<Option<String>>,
}

impl WebkitView {
    fn emit(&self, event: EngineEvent) {
        for listener in self.listeners.borrow().iter() {
            listener(event.clone());
        }
    }

    /// Connect all WebKit signals we care about, translating each into a
    /// [`kaze_engine::EngineEvent`]. This is the single place where
    /// "WebKit's vocabulary" gets translated into "Kaze's vocabulary" —
    /// nothing outside this function should ever need to know WebKit
    /// signal names.
    fn wire_signals(view: &Rc<Self>) {
        let v = view.clone();
        view.web_view.connect_load_changed(move |_, event| {
            let mapped = match event {
                LoadEvent::Started => EngineEvent::LoadStarted,
                LoadEvent::Committed => return, // no direct Kaze equivalent yet
                LoadEvent::Finished => EngineEvent::LoadFinished,
                _ => return,
            };
            v.emit(mapped);
        });

        let v = view.clone();
        view.web_view.connect_load_failed(move |_, _event, uri, error| {
            tracing::warn!(%uri, error = %error, "page load failed");
            v.emit(EngineEvent::LoadFailed {
                message: error.to_string(),
            });
            // Returning true tells WebKit we handled the failure (e.g.
            // would show a custom error page); false lets WebKit show
            // its own. Kaze doesn't have a custom error page in v0.1.
            false
        });

        let v = view.clone();
        view.web_view.connect_estimated_load_progress_notify(move |wv| {
            v.emit(EngineEvent::LoadProgress(wv.estimated_load_progress()));
        });

        let v = view.clone();
        view.web_view.connect_title_notify(move |wv| {
            if let Some(title) = wv.title() {
                v.emit(EngineEvent::TitleChanged(title.to_string()));
            }
        });

        let v = view.clone();
        view.web_view.connect_uri_notify(move |wv| {
            if let Some(uri) = wv.uri() {
                v.emit(EngineEvent::UrlChanged(uri.to_string()));
            }
        });

        let v = view.clone();
        view.web_view.connect_favicon_notify(move |wv| {
            if let Some(texture) = wv.favicon() {
                // gdk4 0.9's Texture has no in-memory PNG-bytes getter,
                // only `save_to_png(path)` — so we round-trip through a
                // scratch file. Fine for favicon-sized images; would be
                // worth revisiting (e.g. via gdk_pixbuf's buffer APIs
                // directly) if this shows up as a hot path.
                let tmp = std::env::temp_dir().join(format!("kaze-favicon-{}.png", uuid::Uuid::new_v4()));
                if texture.save_to_png(&tmp).is_ok() {
                    if let Ok(bytes) = std::fs::read(&tmp) {
                        v.emit(EngineEvent::FaviconChanged(bytes));
                    }
                    let _ = std::fs::remove_file(&tmp);
                }
            }
        });

        // NOTE: the "create" signal (popup / target="_blank" / window.open
        // requests) is intentionally NOT wired in v0.1. In webkit6 0.4's
        // bindings its callback must return a real `gtk::Widget`, which
        // implies actually constructing a WebView for WebKit to drive —
        // there's no "just notify me and let the UI decide" escape hatch
        // at this binding version. Kaze v0.1 therefore lets such
        // navigations fall through to WebKit's default handling (opens
        // in the same view) rather than half-implementing popup support;
        // proper new-tab-on-popup is tracked as follow-up work once
        // either the bindings gain an Option-returning variant or we
        // build the minimal placeholder-widget dance ourselves.
    }
}

/// Thin `dyn EngineView` wrapper around `Rc<WebkitView>`. Kept separate
/// from `WebkitView` itself so the trait impl block stays focused on
/// translating the `EngineView` interface to WebKit calls, while
/// `WebkitView` owns the actual signal-wiring logic above.
struct WebkitViewHandle(Rc<WebkitView>);

impl EngineView for WebkitViewHandle {
    fn load_url(&self, url: &str) {
        *self.0.suspended.borrow_mut() = false;
        self.0.web_view.load_uri(url);
    }

    fn go_back(&self) {
        self.0.web_view.go_back();
    }

    fn go_forward(&self) {
        self.0.web_view.go_forward();
    }

    fn reload(&self) {
        self.0.web_view.reload();
    }

    fn stop_loading(&self) {
        self.0.web_view.stop_loading();
    }

    fn can_go_back(&self) -> bool {
        self.0.web_view.can_go_back()
    }

    fn can_go_forward(&self) -> bool {
        self.0.web_view.can_go_forward()
    }

    fn widget(&self) -> &gtk4::Widget {
        self.0.web_view.upcast_ref::<gtk4::Widget>()
    }

    fn on_event(&self, callback: EngineEventCallback) {
        self.0.listeners.borrow_mut().push(callback);
    }

    fn suspend(&self) {
        // WebKitGTK has no first-class "suspend" API exposed through
        // webkit6 today, so v0.1 approximates it: remember the current
        // URL, then navigate the (still-live) WebView to `about:blank`
        // to drop its rendered content/JS heap. `load_url` above clears
        // the suspended flag and reloads on next activation. This is a
        // deliberate, documented approximation — a true process-level
        // suspend would need lower-level WebKit process-management APIs
        // not yet exposed in these bindings; worth revisiting as the
        // webkit6 crate matures.
        if *self.0.suspended.borrow() {
            return;
        }
        if let Some(uri) = self.0.web_view.uri() {
            *self.0.suspended_url.borrow_mut() = Some(uri.to_string());
        }
        *self.0.suspended.borrow_mut() = true;
        self.0.web_view.load_uri("about:blank");
    }

    fn is_suspended(&self) -> bool {
        *self.0.suspended.borrow()
    }
}
