//! `kaze-ui` — GTK4 + Libadwaita chrome. Depends only on `kaze-engine`'s
//! traits (never `kaze-engine-webkit` directly — see architecture doc
//! §3), and treats `kaze-tabs::TabStore` as the source of truth for
//! everything tab-related, per §4's unidirectional data flow.

pub mod sidebar;
pub mod theme_apply;
pub mod toolbar;
pub mod window;
pub mod newtab;

pub use theme_apply::{install_live_theme, ThemeApplier};
pub use window::BrowserWindow;
