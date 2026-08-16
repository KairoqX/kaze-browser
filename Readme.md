# Kaze — v0.1 Build Status

This is a real, compiling, running Cargo workspace — not a scaffold. Everything
described below was verified by actually building and running the code, not
just by writing it. Unzip and `cargo build --workspace` from a Linux machine
with GTK4 + Libadwaita + WebKitGTK 6.0 dev headers installed.

## What's implemented and verified

| Crate | Status | Verified by |
|---|---|---|
| `kaze-utils` | Done | `cargo test` — paths, errors, logging |
| `kaze-settings` | Done | `cargo test` (3 tests) — TOML persistence, live-reload, migrations scaffold |
| `kaze-theme` | Done | `cargo test` (6 tests) — settings → CSS, light/dark resolution, hex parsing |
| `kaze-tabs` | Done | `cargo test` (8 tests) — open/close/reorder/activate, including a **regression test for a real reentrant-`RefCell` bug found by actually running the app** |
| `kaze-engine` | Done | Compiles against real `gtk4` 0.9 — the `RenderEngine`/`EngineView` trait boundary |
| `kaze-engine-webkit` | Done | Compiles against real WebKitGTK 6.0 headers; wires load/title/url/favicon/progress signals; per-profile `WebContext` isolation for incognito |
| `kaze-ui` | Done | `cargo test` (4 tests) + **actually launched under Xvfb** — sidebar, toolbar, window all render and respond to clicks |
| `kaze-app` | Done | The `kaze` binary builds and **runs** — see screenshots |
| `kaze-history`, `kaze-bookmarks`, `kaze-downloads`, `kaze-session`, `kaze-adblock`, `kaze-network` | **Placeholder only** | Empty `lib.rs` stubs so the workspace resolves — not yet implemented |

**21/21 unit tests pass.** The app was launched under a virtual X display
(Xvfb), screenshotted mid-session, and interacted with via synthetic clicks
(new-tab button) without crashing.

## A real bug found and fixed

`TabStore` originally dispatched `TabEvent`s synchronously to subscriber
closures from inside its mutation methods. The first time the app was
actually *run* (not just compiled), this caused an immediate panic:
`RefCell already mutably borrowed`, because `BrowserWindow` holds
`TabStore` behind `Rc<RefCell<TabStore>>`, and a subscriber callback tried
to `.borrow()` the store while the mutating call still held `.borrow_mut()`.

Fixed by changing `TabStore` to *queue* events (`take_events()`) rather than
dispatching them inline, and updating `kaze-ui` to drain the queue only
after each mutating borrow has been released. A regression test
(`events_can_be_drained_after_releasing_the_mutable_borrow`) now guards this
specific pattern. See the doc comment at the top of `kaze-tabs/src/store.rs`
for the full writeup — this is exactly the kind of bug that `cargo build`
alone cannot catch, which is why the app was launched and screenshotted
rather than just compiled.

## Toolchain notes for whoever builds this next

- This sandbox's `apt` only has rustc/cargo up to **1.91.1**. The workspace
  is pinned to `gtk4 = "0.9"` and `webkit6 = "0.4"` because the newer
  `gtk4-rs` 0.11/`webkit6` 0.6 pairing requires rustc 1.92+. **If you have a
  newer toolchain available, bumping these is worth doing** — nothing in the
  architecture depends on the specific binding version, and newer bindings
  will have better API coverage (e.g. an `Option`-returning `connect_create`
  signal for proper popup handling, and `Texture::save_to_png_bytes` instead
  of the temp-file round-trip currently used for favicons).
- Install the dev headers with something like:
  `apt-get install -y libgtk-4-dev libadwaita-1-dev libwebkitgtk-6.0-dev`

## Known, documented compromises in this pass

1. **Popup / `window.open` handling is not wired.** The `webkit6` 0.4
   bindings' `connect_create` signal requires returning a real `gtk::Widget`
   rather than `Option<Widget>`, so there's no clean "just notify me" path
   at this binding version. Left unconnected rather than half-implemented;
   see the comment in `kaze-engine-webkit/src/lib.rs`.
2. **Tab suspension is an approximation.** True process-level suspend isn't
   exposed through these bindings yet; v0.1 navigates suspended tabs to
   `about:blank` and remembers the URL to reload on reactivation. Documented
   in `EngineView::suspend`.
3. **Sidebar rebuilds its row list on every `TabEvent`** rather than using a
   virtualized `gio::ListModel`-backed `GtkListView` as described in the
   architecture doc. Correct for v0.1 tab counts; worth revisiting before
   tab counts get large. Documented in `kaze-ui/src/sidebar.rs`.
4. **No real new-tab page yet** — homepage defaults to an inline `data:`
   URL. A proper `about:newtab`-style page needs a registered custom URI
   scheme handler in `kaze-engine-webkit`, not yet built.
5. **`kaze-history`, `kaze-bookmarks`, `kaze-downloads`, `kaze-session`,
   `kaze-adblock`, `kaze-network`** are empty placeholder crates. The
   architecture for each is designed (see `ARCHITECTURE.md`) but none are
   implemented yet.

## Suggested next session's priorities

1. Implement `kaze-history` (SQLite via `rusqlite`) and wire
   `EngineEvent::UrlChanged` → history writes (skipping incognito profiles).
2. Implement `kaze-bookmarks` similarly.
3. Implement `kaze-downloads`, hooking `EngineEvent::DownloadRequested`
   (currently emitted but unhandled).
4. Build the `AdwPreferencesWindow` settings UI bound to `KazeSettings`.
5. Swap the sidebar to a real `gio::ListModel` + `GtkListView`.
