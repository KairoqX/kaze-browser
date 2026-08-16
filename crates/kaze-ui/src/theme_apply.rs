//! Bridges `kaze-theme`'s plain CSS `String` output into a real
//! `gtk4::CssProvider` applied to the default `Display` — the one place
//! in the whole codebase that touches GTK's styling APIs directly. See
//! architecture doc §10.

use gtk4::gdk::Display;
use gtk4::CssProvider;
use kaze_settings::ThemeSettings;
use std::cell::RefCell;
use std::rc::Rc;

pub struct ThemeApplier {
    provider: CssProvider,
}

impl ThemeApplier {
    pub fn new() -> Self {
        let provider = CssProvider::new();
        if let Some(display) = Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        Self { provider }
    }

    /// Re-resolve `settings` against the current system dark-mode state
    /// and push the regenerated CSS into the live provider. Call this
    /// once at startup and again every time settings change or the
    /// system theme flips — see kaze-theme's module docs.
    pub fn apply(&self, settings: &ThemeSettings, system_is_dark: bool) {
        let resolved = kaze_theme::resolve(settings, system_is_dark);
        let css = kaze_theme::generate_css(&resolved);
        self.provider.load_from_string(&css);
    }
}

impl Default for ThemeApplier {
    fn default() -> Self {
        Self::new()
    }
}

/// Wires a `ThemeApplier` to both `SettingsStore` changes and
/// `Adw::StyleManager`'s dark-mode signal, so accent color / corner
/// radius / blur / etc. all repaint live with zero relaunch — the
/// concrete implementation of the "Live updates" guarantee in the
/// architecture doc's theme engine section.
pub fn install_live_theme(
    applier: Rc<ThemeApplier>,
    settings: Rc<RefCell<kaze_settings::SettingsStore>>,
) {
    let style_manager = libadwaita::StyleManager::default();

    let apply_now = {
        let applier = applier.clone();
        let settings = settings.clone();
        let style_manager = style_manager.clone();
        move || {
            let theme = settings.borrow().current().theme.clone();
            applier.apply(&theme, style_manager.is_dark());
        }
    };

    apply_now();

    // System dark-mode toggles (e.g. user flips their GNOME theme).
    {
        let apply_now = apply_now.clone();
        style_manager.connect_dark_notify(move |_| apply_now());
    }

    // Settings changes (e.g. user picks a new accent color in prefs).
    // Note: SettingsStore::subscribe takes ownership of the closure and
    // is called synchronously from `update()`, matching the
    // unidirectional data flow described in architecture doc §4.
    settings.borrow_mut().subscribe(Box::new(move |_new_settings| {
        apply_now();
    }));
}
