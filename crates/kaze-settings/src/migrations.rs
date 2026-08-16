//! Schema migrations. Each function moves a raw TOML value forward by
//! exactly one version. `load_and_migrate` in `store.rs` walks a loaded
//! document through every migration needed to reach
//! [`crate::schema::CURRENT_SCHEMA_VERSION`] before deserializing into
//! [`crate::schema::KazeSettings`].
//!
//! There is deliberately no migration registered yet — schema version 1
//! is the first shipped version. This file exists so the *pattern* is
//! established before it's needed, which is what makes future migrations
//! additive instead of a refactor.

use toml::Value;

/// Applies every migration needed to bring `doc` up to
/// `CURRENT_SCHEMA_VERSION`. Unknown/missing `schema_version` is treated
/// as version 1 (the first shipped schema) rather than an error, so a
/// hand-edited file without that key still loads.
pub fn migrate(doc: Value) -> Value {
    let _version = doc
        .get("schema_version")
        .and_then(Value::as_integer)
        .unwrap_or(1) as u32;

    // Example of how a future migration would be wired in (note `doc`
    // would need to become `mut doc` again once this is uncommented):
    //
    // if _version == 1 {
    //     migrate_v1_to_v2(&mut doc);
    // }

    doc
}
