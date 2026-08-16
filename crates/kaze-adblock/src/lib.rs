//! `kaze-adblock` — compiles a curated blocklist into WebKit's native
//! content-blocker JSON format. See architecture doc §11. This crate
//! only *generates* the ruleset; `kaze-engine-webkit` is responsible for
//! compiling it via `WebKitUserContentFilterStore` and attaching it to
//! a `UserContentManager`, since that's WebKit-specific machinery this
//! crate deliberately has no dependency on.

pub mod domains;
pub mod rules;

pub use rules::build_ruleset_json;