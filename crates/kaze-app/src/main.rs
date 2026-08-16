//! `kaze-app` — the composition root. Per architecture doc §3: this
//! binary should stay thin, almost no logic of its own, just wiring
//! together the settings store, theme engine, render engine, and UI
//! layer in the order described in §5 (Browser Lifecycle).

use gtk4::glib;
use gtk4::prelude::*;
use kaze_engine::RenderEngine;
use kaze_engine_webkit::WebkitEngine;
use kaze_ui::BrowserWindow;
use libadwaita::prelude::ApplicationExtManual;
use std::cell::RefCell;
use std::rc::Rc;

const APP_ID: &str = "org.kaze.Browser";

fn main() -> glib::ExitCode {
    kaze_utils::logging::init();
    tracing::info!("starting Kaze");

    let paths = match kaze_utils::KazePaths::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("fatal: could not resolve platform directories: {e}");
            std::process::exit(1);
        }
    };
    if let Err(e) = paths.ensure_dirs() {
        eprintln!("fatal: could not create config/data/cache dirs: {e}");
        std::process::exit(1);
    }

    let settings = match kaze_settings::SettingsStore::load(paths.settings_file()) {
        Ok(s) => Rc::new(RefCell::new(s)),
        Err(e) => {
            tracing::error!(error = %e, "failed to load settings, aborting");
            std::process::exit(1);
        }
    };

    let app = libadwaita::Application::builder().application_id(APP_ID).build();

    app.connect_activate(move |app| {
        tracing::info!("activating Kaze window");

        // Theme engine wiring — see kaze-ui::theme_apply and
        // architecture doc §10. Installed once per activation; GTK only
        // calls `activate` once per process for a single-window app.
        let theme_applier = Rc::new(kaze_ui::ThemeApplier::new());
        kaze_ui::install_live_theme(theme_applier, settings.clone());

        // Render engine — the ONLY place `kaze-engine-webkit` is named
        // outside of that crate itself, per architecture doc §3.
        let engine: Rc<dyn RenderEngine> = Rc::new(WebkitEngine::new());
        tracing::info!(engine = %engine.name(), "render engine ready");

        let current = settings.borrow().current().clone();
        let window = BrowserWindow::new(
            app,
            engine,
            current.general.homepage.clone(),
            current.theme.sidebar_width_px,
        );

        window.window.present();
    });

    app.run()
}
