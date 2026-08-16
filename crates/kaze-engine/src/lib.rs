//! `kaze-engine` — the render-engine abstraction described in §3 and §6
//! of the architecture doc.
//!
//! This is the single most load-bearing boundary in the codebase:
//! `kaze-ui` depends ONLY on the traits in this file, never on
//! `kaze-engine-webkit` directly. That wiring happens exactly once, in
//! `kaze-app`'s composition root. If another engine ever needs to exist
//! (Servo, WebView2 on Windows — see architecture doc §13), it implements
//! these same two traits and nothing above this layer needs to change.
//!
//! The only GTK dependency this crate has is the `gtk4::Widget` return
//! type on [`EngineView::widget`] — because an engine view fundamentally
//! *is* an embeddable GTK widget wrapping whatever the engine renders
//! into. Everything else here is engine-agnostic data.

use kaze_tabs::ProfileId;

/// Configuration for creating a new engine-backed view (one per tab).
#[derive(Debug, Clone)]
pub struct ViewConfig {
    pub profile: ProfileId,
    pub initial_url: String,
    pub enable_javascript: bool,
    /// `None` means "use the engine's default UA". Kaze doesn't spoof a
    /// UA by default — see privacy notes in kaze-network — but power
    /// users may override it per-profile.
    pub user_agent: Option<String>,
}

/// Events an [`EngineView`] can emit. `kaze-tabs::TabStore::update` is
/// typically called in response to these, translating engine-level
/// signals into tab-model state.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    LoadStarted,
    /// 0.0..=1.0
    LoadProgress(f64),
    LoadFinished,
    LoadFailed { message: String },
    TitleChanged(String),
    UrlChanged(String),
    FaviconChanged(Vec<u8>),
    /// The page tried to open a new window/tab (e.g. `target="_blank"`,
    /// `window.open`). The UI decides whether that becomes a new tab, a
    /// new window, or is blocked (e.g. popup blocking policy).
    NewWindowRequested { url: String },
    DownloadRequested {
        url: String,
        suggested_filename: Option<String>,
    },
    /// Emitted when the engine believes navigation should be upgraded to
    /// HTTPS but wants confirmation the policy layer agrees (kaze-network
    /// owns the actual decision; the engine surfaces the opportunity).
    InsecureNavigationAttempted { url: String },
}

pub type EngineEventCallback = Box<dyn Fn(EngineEvent)>;

/// One tab's live content view. Implementations wrap a real engine's
/// widget (e.g. `webkit6::WebView`) behind this interface.
pub trait EngineView {
    fn load_url(&self, url: &str);
    fn go_back(&self);
    fn go_forward(&self);
    fn reload(&self);
    fn stop_loading(&self);

    fn can_go_back(&self) -> bool;
    fn can_go_forward(&self) -> bool;

    /// The embeddable widget for this view. `kaze-ui` places this inside
    /// the tab content area. Implementations must return the *same*
    /// widget instance across calls (this is a getter, not a factory).
    fn widget(&self) -> &gtk4::Widget;

    /// Register a callback for engine events. Multiple subscribers are
    /// allowed; typically `kaze-tabs` and `kaze-downloads` both listen.
    fn on_event(&self, callback: EngineEventCallback);

    /// Release the backing native view to reclaim memory while keeping
    /// the tab's metadata (url, title, favicon) intact — see §6 "tab
    /// suspension" and the `suspend_inactive_tabs` setting. Calling
    /// `load_url` or otherwise reactivating after suspension must
    /// transparently recreate the underlying view.
    fn suspend(&self);
    fn is_suspended(&self) -> bool;
}

/// Factory for [`EngineView`]s. One `RenderEngine` implementation is
/// constructed at startup (in `kaze-app`) and handed down to whatever in
/// `kaze-ui` creates tabs.
pub trait RenderEngine {
    fn create_view(&self, config: ViewConfig) -> Box<dyn EngineView>;

    /// Human-readable engine name, surfaced in `about:kaze` / diagnostics
    /// (e.g. "WebKitGTK 2.52.3"). Never used for feature-detection logic —
    /// code should never branch on engine name; that defeats the point
    /// of this abstraction.
    fn name(&self) -> String;
}
