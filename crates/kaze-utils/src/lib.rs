//! `kaze-utils` — shared, browser-agnostic building blocks.
//!
//! This crate intentionally has no GTK, no WebKit, and no browser-domain
//! knowledge (no concept of a "tab" or "bookmark"). If you find yourself
//! wanting to add something browser-specific here, it belongs in a more
//! specific crate instead.

pub mod error;
pub mod logging;
pub mod paths;

pub use error::{KazeError, Result};
pub use paths::KazePaths;
