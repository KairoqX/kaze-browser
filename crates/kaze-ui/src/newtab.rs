//! The internal new-tab page. Kept as a small, self-contained HTML
//! document rather than a real `about:` URI scheme — WebKitGTK doesn't
//! resolve app-defined `about:` pages without a registered custom URI
//! scheme handler (discovered the hard way: it just showed "The URL
//! can't be shown"). Until that scheme handler exists, `kaze-ui` uses a
//! symbolic sentinel internally and only turns it into a real `data:`
//! URL right before handing it to the engine — this keeps the sentinel
//! out of settings/history/the address bar, where a raw `data:` URL
//! would be ugly and confusing to show a user.

use base64::{engine::general_purpose::STANDARD, Engine};

/// What `kaze-settings`'s default homepage and the sidebar's "+" button
/// both refer to. Never handed directly to `EngineView::load_url` —
/// always resolve it first via [`resolve`].
pub const NEWTAB_SENTINEL: &str = "kaze://newtab";

pub fn is_internal_page(url: &str) -> bool {
    url == NEWTAB_SENTINEL || url.starts_with("data:text/html;base64,")
}

/// Turn the sentinel into a real, loadable `data:` URL. Any other URL
/// passes through unchanged, so callers can unconditionally route
/// through this function.
pub fn resolve(url: &str) -> String {
    if url != NEWTAB_SENTINEL {
        return url.to_string();
    }
    let encoded = STANDARD.encode(NEWTAB_HTML);
    format!("data:text/html;base64,{encoded}")
}

const NEWTAB_HTML: &str = r#"<!doctype html>
<html>
<head>
<title>New Tab</title>
<style>
  html, body { margin: 0; height: 100%; }
  body {
    font-family: -apple-system, "Segoe UI", sans-serif;
    background: #fafafa;
    color: #1a1a1a;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }
  h1 {
    font-size: 40px;
    font-weight: 700;
    color: #7c6cf0;
    margin: 0 0 28px 0;
    letter-spacing: -0.02em;
  }
  form { width: 100%; max-width: 480px; padding: 0 24px; }
  input {
    width: 100%;
    box-sizing: border-box;
    padding: 14px 20px;
    font-size: 16px;
    border-radius: 999px;
    border: 1px solid #ddd;
    outline: none;
    background: #fff;
  }
  input:focus { border-color: #7c6cf0; }
</style>
</head>
<body>
  <h1>Kaze</h1>
  <form onsubmit="
    event.preventDefault();
    var q = document.getElementById('q').value.trim();
    if (q) { window.location.href = 'https://duckduckgo.com/?q=' + encodeURIComponent(q); }
    return false;
  ">
    <input id="q" type="text" placeholder="Search the web" autofocus autocomplete="off">
  </form>
</body>
</html>"#;