//! Centralized `tracing` setup so every crate logs consistently and
//! nothing sets up its own competing subscriber.

use tracing_subscriber::{fmt, EnvFilter};

/// Initialize the global tracing subscriber. Call once, from `kaze-app`'s
/// `main()`, before anything else runs.
///
/// Honors `RUST_LOG` if set (e.g. `RUST_LOG=kaze_adblock=debug`), defaults
/// to `info` otherwise. Deliberately simple: no telemetry, no remote
/// exporters, no analytics sink — this writes to stderr and nowhere else,
/// in keeping with Kaze's "no telemetry" principle.
pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .compact()
        .init();
}
