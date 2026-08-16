//! A curated list of common ad/tracker domains. This is NOT a full
//! EasyList/EasyPrivacy parser — see the architecture doc §11 for the
//! long-term plan (parsing real filter lists into this same JSON
//! output). This list exists so v0.1 has a real, working blocker rather
//! than none at all, and is meant to be replaced/extended by a proper
//! filter-list pipeline later without changing the JSON generation code
//! in `rules.rs`.

pub const BLOCKED_DOMAINS: &[&str] = &[
    // Ad networks / exchanges
    "doubleclick.net", "googlesyndication.com", "googleadservices.com",
    "adservice.google.com", "adsafeprotected.com", "amazon-adsystem.com",
    "adnxs.com", "rubiconproject.com", "pubmatic.com", "openx.net",
    "casalemedia.com", "indexexchange.com", "bidswitch.net",
    "smartadserver.com", "criteo.com", "criteo.net", "taboola.com",
    "outbrain.com", "mgid.com", "revcontent.com", "popads.net",
    "propellerads.com", "adroll.com", "media.net", "sharethrough.com",
    "teads.tv", "yieldmo.com", "connatix.com", "moatads.com",
    "adform.net", "adcolony.com", "applovin.com", "unityads.unity3d.com",

    // Analytics / tracking
    "google-analytics.com", "googletagmanager.com", "googletagservices.com",
    "scorecardresearch.com", "quantserve.com", "hotjar.com", "mixpanel.com",
    "segment.io", "segment.com", "doubleverify.com", "chartbeat.com",
    "chartbeat.net", "newrelic.com", "nr-data.net", "amplitude.com",
    "fullstory.com", "mouseflow.com", "crazyegg.com", "clicktale.net",
    "bluekai.com", "krxd.net", "demdex.net", "everesttech.net",
    "adsrvr.org", "agkn.com", "rlcdn.com", "tapad.com",

    // Social widgets (tracking-heavy embeds)
    "connect.facebook.net", "facebook.com/tr", "platform.twitter.com",
    "ads-twitter.com", "analytics.twitter.com", "pinterest.com/ct",
    "ads.linkedin.com", "px.ads.linkedin.com", "snap.licdn.com",

    // Misc widely-blocked
    "adsystem.com", "advertising.com", "yieldlab.net", "sitescout.com",
    "contextweb.com", "adtechus.com", "zedo.com", "adtelligent.com",
    "gumgum.com", "sovrn.com", "lijit.com", "spotxchange.com",
];

/// Common cosmetic ad-container selectors, hidden via CSS on every page
/// regardless of domain. Deliberately conservative — broad selectors
/// like `[class*="ad"]` cause false positives on legitimate content
/// (e.g. "add", "adobe", "adventure"), so this list sticks to
/// well-established, low-collision ad-slot conventions.
pub const COSMETIC_SELECTORS: &[&str] = &[
    "div[id^=\"google_ads_iframe\"]",
    "ins.adsbygoogle",
    "div.adsbygoogle",
    "div[id^=\"div-gpt-ad\"]",
    "iframe[id^=\"google_ads_iframe\"]",
    "div.ad-container",
    "div.advertisement",
    "aside.ad-slot",
];