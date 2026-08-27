use std::collections::HashMap;

use scanner_core::EnhancementPreset;
use serde::{Deserialize, Serialize};

const EN_US: &str = include_str!("../../../assets/locales/en-US.yaml");
const ZH_CN: &str = include_str!("../../../assets/locales/zh-CN.yaml");

/// User-selected language setting. `Auto` follows the system locale.
#[derive(
    Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash,
)]
#[serde(rename_all = "lowercase")]
pub enum LanguagePreference {
    #[serde(rename = "auto", alias = "system")]
    #[default]
    Auto,
    #[serde(rename = "en-US")]
    English,
    #[serde(rename = "zh-CN")]
    Chinese,
}

/// Resolved language actually used for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Chinese,
}

impl LanguagePreference {
    pub const ALL: [Self; 3] = [Self::Auto, Self::English, Self::Chinese];

    pub fn resolve(self) -> Language {
        match self {
            Self::English => Language::English,
            Self::Chinese => Language::Chinese,
            Self::Auto => detect_system_language(),
        }
    }

    /// Display name for this option, always rendered in the active language
    /// except for language names, which use their native spelling.
    pub fn label(self, i18n: &I18n) -> String {
        i18n.tr(match self {
            Self::Auto => "language.auto",
            Self::English => "language.english",
            Self::Chinese => "language.chinese",
        })
    }
}

/// Message catalog for the currently resolved language.
#[derive(Debug, Clone)]
pub struct I18n {
    preference: LanguagePreference,
    language: Language,
    messages: HashMap<String, String>,
}

impl I18n {
    pub fn new(preference: LanguagePreference) -> Self {
        let language = preference.resolve();
        Self {
            preference,
            language,
            messages: load_messages(language),
        }
    }

    pub fn preference(&self) -> LanguagePreference {
        self.preference
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn set_preference(&mut self, preference: LanguagePreference) {
        if self.preference == preference {
            return;
        }
        self.preference = preference;
        self.language = preference.resolve();
        self.messages = load_messages(self.language);
    }

    /// Looks up a message. Unknown keys render as the key itself so a
    /// missing translation is visible instead of producing blank UI.
    ///
    /// Returns an owned `String` (rather than a borrowed `&str`) so callers
    /// can hold the result while mutably borrowing the application state
    /// inside egui panel closures.
    pub fn tr(&self, key: &str) -> String {
        self.messages
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }

    /// Looks up a message and substitutes `{name}` placeholders.
    pub fn text(&self, key: &str, values: &[(&str, String)]) -> String {
        let mut message = self.tr(key);
        for (name, value) in values {
            message = message.replace(&format!("{{{name}}}"), value);
        }
        message
    }

    pub fn preset_label(&self, preset: EnhancementPreset) -> String {
        self.tr(match preset {
            EnhancementPreset::Original => "preset.original",
            EnhancementPreset::AdaptiveBlackAndWhite => {
                "preset.adaptive_black_and_white"
            }
            EnhancementPreset::EnhancedColor => "preset.enhanced_color",
            EnhancementPreset::MagicColor => "preset.magic_color",
        })
    }
}

fn load_messages(language: Language) -> HashMap<String, String> {
    let yaml = match language {
        Language::English => EN_US,
        Language::Chinese => ZH_CN,
    };
    // Locale files are build-time constants; a parse failure is a developer
    // error, not user input.
    let value: serde_yaml::Value = serde_yaml::from_str(yaml)
        .expect("Built-in locale resource is malformed.");
    let mut messages = HashMap::new();
    flatten_messages(String::new(), &value, &mut messages);
    messages
}

fn flatten_messages(
    prefix: String,
    value: &serde_yaml::Value,
    messages: &mut HashMap<String, String>,
) {
    match value {
        serde_yaml::Value::Mapping(mapping) => {
            for (key, value) in mapping {
                let Some(key) = key.as_str() else {
                    continue;
                };
                let full_key = if prefix.is_empty() {
                    key.to_owned()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_messages(full_key, value, messages);
            }
        }
        serde_yaml::Value::String(message) => {
            messages.insert(prefix, message.clone());
        }
        _ => {}
    }
}

fn detect_system_language() -> Language {
    let locale = sys_locale::get_locale()
        .or_else(|| {
            ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"]
                .iter()
                .filter_map(|name| std::env::var(name).ok())
                .next()
        })
        .unwrap_or_default()
        .to_ascii_lowercase();

    if locale.starts_with("zh") {
        Language::Chinese
    } else {
        Language::English
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_resources_have_the_same_keys() {
        let english = load_messages(Language::English);
        let chinese = load_messages(Language::Chinese);
        assert_eq!(english.len(), chinese.len());
        let missing: Vec<&String> = english
            .keys()
            .filter(|key| !chinese.contains_key(*key))
            .collect();
        assert!(missing.is_empty(), "zh-CN is missing keys: {missing:?}");
        let extra: Vec<&String> = chinese
            .keys()
            .filter(|key| !english.contains_key(*key))
            .collect();
        assert!(extra.is_empty(), "zh-CN has unknown keys: {extra:?}");
    }

    #[test]
    fn locale_values_are_never_empty() {
        for language in [Language::English, Language::Chinese] {
            for (key, value) in load_messages(language) {
                assert!(!value.trim().is_empty(), "{key} is empty");
            }
        }
    }

    #[test]
    fn interpolation_replaces_named_values() {
        let i18n = I18n::new(LanguagePreference::English);
        let text = i18n
            .text("messages.exported", &[("file", "page-01.png".to_owned())]);
        assert_eq!(text, "Exported page-01.png");
    }

    #[test]
    fn interpolation_keeps_placeholders_for_missing_values() {
        let i18n = I18n::new(LanguagePreference::English);
        let text = i18n.text("messages.session", &[]);
        assert_eq!(text, "Session: {file}");
    }

    #[test]
    fn unknown_keys_fall_back_to_the_key() {
        let i18n = I18n::new(LanguagePreference::Chinese);
        assert_eq!(i18n.tr("does.not.exist"), "does.not.exist");
    }

    #[test]
    fn every_enhancement_preset_has_a_label() {
        let i18n = I18n::new(LanguagePreference::English);
        for preset in EnhancementPreset::ALL {
            let label = i18n.preset_label(preset);
            assert!(
                !label.starts_with("preset."),
                "missing label for {preset:?}"
            );
            assert!(!label.trim().is_empty());
        }
    }

    #[test]
    fn explicit_preference_overrides_auto() {
        let i18n = I18n::new(LanguagePreference::Chinese);
        assert_eq!(i18n.language(), Language::Chinese);
        assert_eq!(i18n.preference(), LanguagePreference::Chinese);
    }
}
