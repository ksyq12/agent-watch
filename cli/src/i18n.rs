//! Internationalization support using fluent-rs.
//!
//! Provides message lookup for CLI strings, defaulting to English.

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use std::sync::OnceLock;
use unic_langid::LanguageIdentifier;

const EN_MESSAGES: &str = include_str!("../locales/en/main.ftl");

static BUNDLE: OnceLock<FluentBundle<FluentResource>> = OnceLock::new();

fn get_bundle() -> &'static FluentBundle<FluentResource> {
    BUNDLE.get_or_init(|| {
        let langid: LanguageIdentifier = "en-US".parse().expect("valid language identifier");
        let mut bundle = FluentBundle::new_concurrent(vec![langid]);
        let resource =
            FluentResource::try_new(EN_MESSAGES.to_string()).expect("valid FTL resource");
        bundle
            .add_resource(resource)
            .expect("no conflicting resources");
        bundle
    })
}

/// Look up a message by its identifier. Returns the id itself if not found.
pub fn t(id: &str) -> String {
    let bundle = get_bundle();
    let Some(msg) = bundle.get_message(id) else {
        return id.to_string();
    };
    let Some(pattern) = msg.value() else {
        return id.to_string();
    };
    let mut errors = vec![];
    bundle
        .format_pattern(pattern, None, &mut errors)
        .to_string()
}

/// Look up a message with named arguments.
pub fn t_args(id: &str, args: &[(&str, &str)]) -> String {
    let bundle = get_bundle();
    let Some(msg) = bundle.get_message(id) else {
        return id.to_string();
    };
    let Some(pattern) = msg.value() else {
        return id.to_string();
    };
    let mut fluent_args = FluentArgs::new();
    for (key, val) in args {
        fluent_args.set(*key, *val);
    }
    let mut errors = vec![];
    bundle
        .format_pattern(pattern, Some(&fluent_args), &mut errors)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_lookup() {
        let result = t("version-title");
        assert_eq!(result, "MacAgentWatch");
    }

    #[test]
    fn test_lookup_with_args() {
        let result = t_args("session-ended", &[("exit_code", "0")]);
        assert_eq!(result, "Session ended (exit: \u{2068}0\u{2069})");
    }

    #[test]
    fn test_missing_key_returns_id() {
        let result = t("nonexistent-key");
        assert_eq!(result, "nonexistent-key");
    }

    #[test]
    fn test_risk_levels() {
        assert_eq!(t("risk-low"), "LOW");
        assert_eq!(t("risk-medium"), "MEDIUM");
        assert_eq!(t("risk-high"), "HIGH");
        assert_eq!(t("risk-critical"), "CRITICAL");
    }
}
