//! `kaze-tabs` — the tab data model and state machine described in §7 of
//! the architecture doc. No GTK dependency: `kaze-ui`'s sidebar binds to
//! this via a `gio::ListModel` adapter, but this crate has no idea GTK
//! exists, which is what makes "open/close/reorder N tabs" testable in
//! milliseconds without spinning up a display.

pub mod model;
pub mod store;

pub use model::{ProfileId, Tab, TabId};
pub use store::{TabEvent, TabStore};
