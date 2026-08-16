//! Compiles [`domains::BLOCKED_DOMAINS`] and [`domains::COSMETIC_SELECTORS`]
//! into WebKit's native content-blocker JSON format (the same
//! trigger/action dialect Safari uses — WebKitGTK reused Apple's content
//! extension compiler). See architecture doc §11: this is the
//! "network-level" and "cosmetic filtering" layers described there,
//! just with a curated seed list instead of a full EasyList parse.

use crate::domains::{BLOCKED_DOMAINS, COSMETIC_SELECTORS};
use serde_json::{json, Value};

/// Builds the full ruleset as a JSON string, ready to hand to
/// `WebKitUserContentFilterStore::save`.
pub fn build_ruleset_json() -> String {
    let mut rules: Vec<Value> = Vec::with_capacity(BLOCKED_DOMAINS.len() + COSMETIC_SELECTORS.len());

    for domain in BLOCKED_DOMAINS {
        // Escape dots for the regex `url-filter` WebKit expects, and
        // match the domain anywhere in the URL (subdomains included).
        let escaped = domain.replace('.', r"\.");
        rules.push(json!({
            "trigger": {
                "url-filter": format!(".*{escaped}.*"),
                "resource-type": ["document", "image", "style-sheet", "script", "raw", "font", "media"]
            },
            "action": { "type": "block" }
        }));
    }

    for selector in COSMETIC_SELECTORS {
        rules.push(json!({
            "trigger": { "url-filter": ".*" },
            "action": { "type": "css-display-none", "selector": selector }
        }));
    }

    serde_json::to_string(&rules).expect("ruleset is always valid JSON by construction")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_valid_json_array() {
        let json = build_ruleset_json();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
        assert!(parsed.as_array().unwrap().len() > 50);
    }

    #[test]
    fn every_rule_has_trigger_and_action() {
        let json = build_ruleset_json();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        for rule in parsed.as_array().unwrap() {
            assert!(rule.get("trigger").is_some());
            assert!(rule.get("action").is_some());
        }
    }
}