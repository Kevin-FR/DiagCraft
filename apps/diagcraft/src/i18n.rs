//! Interface translation and newly generated welcome content only.
//! Existing drawing text and API identifiers never pass here.
use serde::{Deserialize, Serialize};
use std::{cell::Cell, collections::BTreeMap, sync::OnceLock};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Language {
    #[serde(rename = "en")]
    English,
    #[default]
    #[serde(rename = "fr")]
    French,
    #[serde(rename = "de")]
    German,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "it")]
    Italian,
    #[serde(rename = "pt")]
    Portuguese,
    #[serde(rename = "zh-Hans")]
    Chinese,
}
impl Language {
    pub const ALL: [Self; 7] = [
        Self::English,
        Self::French,
        Self::German,
        Self::Spanish,
        Self::Italian,
        Self::Portuguese,
        Self::Chinese,
    ];
    pub fn native(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::French => "Français",
            Self::German => "Deutsch",
            Self::Spanish => "Español",
            Self::Italian => "Italiano",
            Self::Portuguese => "Português",
            Self::Chinese => "简体中文",
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::French => "fr",
            Self::German => "de",
            Self::Spanish => "es",
            Self::Italian => "it",
            Self::Portuguese => "pt",
            Self::Chinese => "zh-Hans",
        }
    }
    pub fn parse(locale: &str) -> Option<Self> {
        let code = locale
            .trim()
            .split(['_', '-', '.'])
            .next()?
            .to_ascii_lowercase();
        match code.as_str() {
            "en" => Some(Self::English),
            "fr" => Some(Self::French),
            "de" => Some(Self::German),
            "es" => Some(Self::Spanish),
            "it" => Some(Self::Italian),
            "pt" => Some(Self::Portuguese),
            "zh" => Some(Self::Chinese),
            _ => None,
        }
    }
    pub fn system() -> Self {
        static SYSTEM: OnceLock<Language> = OnceLock::new();
        *SYSTEM.get_or_init(Self::detect_system)
    }
    fn detect_system() -> Self {
        for key in ["LC_ALL", "LC_MESSAGES"] {
            if let Ok(locale) = std::env::var(key)
                && let Some(language) = Self::parse(&locale)
            {
                return language;
            }
        }
        #[cfg(target_os = "macos")]
        for domain in ["org.diagcraft.editor", "-g"] {
            if let Ok(output) = std::process::Command::new("/usr/bin/defaults")
                .args(["read", domain, "AppleLanguages"])
                .output()
                && output.status.success()
            {
                for token in
                    String::from_utf8_lossy(&output.stdout).split(['\n', '"', ',', ' ', '(', ')'])
                {
                    if let Some(language) = Self::parse(token) {
                        return language;
                    }
                }
            }
        }
        #[cfg(target_os = "windows")]
        if let Ok(output) = std::process::Command::new(
            std::path::PathBuf::from(
                std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()),
            )
            .join("System32/WindowsPowerShell/v1.0/powershell.exe"),
        )
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[System.Globalization.CultureInfo]::CurrentUICulture.Name",
        ])
        .output()
            && output.status.success()
            && let Some(language) = Self::parse(&String::from_utf8_lossy(&output.stdout))
        {
            return language;
        }
        std::env::var("LANG")
            .ok()
            .and_then(|value| Self::parse(&value))
            .unwrap_or(Self::English)
    }
}
thread_local! { static ACTIVE: Cell<Language> = const { Cell::new(Language::French) }; }
pub(super) fn activate(language: Language) {
    ACTIVE.set(language);
}
pub(super) fn active() -> Language {
    ACTIVE.get()
}
type Catalogue = BTreeMap<String, BTreeMap<String, String>>;
pub(super) fn catalogue() -> &'static Catalogue {
    static CATALOGUE: OnceLock<Catalogue> = OnceLock::new();
    CATALOGUE.get_or_init(|| {
        serde_json::from_str(include_str!("../../../assets/locales/messages.json"))
            .expect("validated interface catalogue")
    })
}
pub(super) fn translate(language: Language, source: &str) -> String {
    catalogue()
        .get(source)
        .and_then(|row| row.get(language.code()))
        .map_or_else(|| source.to_owned(), Clone::clone)
}
pub(super) fn t(source: impl AsRef<str>) -> String {
    translate(active(), source.as_ref())
}

/// Named substitutions preserve user values verbatim; only the template is translated.
pub(super) fn render(source: &str, values: &[(&str, String)]) -> String {
    let template = t(source);
    let mut result = String::with_capacity(template.len());
    let mut tail = template.as_str();
    while let Some(start) = tail.find('{') {
        result.push_str(&tail[..start]);
        let Some(end) = tail[start..].find('}') else {
            result.push_str(&tail[start..]);
            return result;
        };
        let key = &tail[start + 1..start + end];
        if let Some((_, value)) = values.iter().find(|(name, _)| *name == key) {
            result.push_str(value);
        } else {
            result.push_str(&tail[start..=start + end]);
        }
        tail = &tail[start + end + 1..];
    }
    result.push_str(tail);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_variants_and_fallback_preserve_unknown_values() {
        for (locale, expected) in [
            ("de_DE.UTF-8", Language::German),
            ("pt-BR", Language::Portuguese),
            ("zh-CN", Language::Chinese),
            ("en-GB", Language::English),
            (" FR ", Language::French),
        ] {
            assert_eq!(Language::parse(locale), Some(expected));
        }
        assert_eq!(Language::parse("ja-JP"), None);
        assert_eq!(
            translate(Language::Chinese, "My text {not a template}"),
            "My text {not a template}"
        );
    }

    #[test]
    fn every_catalogue_entry_has_seven_languages_and_identical_placeholders() {
        fn placeholders(s: &str) -> Vec<String> {
            let mut keys: Vec<_> = s
                .split('{')
                .skip(1)
                .filter_map(|tail| tail.split_once('}'))
                .map(|(key, _)| key.to_owned())
                .collect();
            keys.sort();
            keys
        }
        assert!(catalogue().len() > 700);
        for (key, row) in catalogue() {
            assert_eq!(row.len(), Language::ALL.len(), "{key}");
            assert_eq!(row["fr"], *key);
            for language in Language::ALL {
                let value = &row[language.code()];
                assert!(!value.trim().is_empty(), "{key}: {language:?}");
                assert_eq!(
                    placeholders(value),
                    placeholders(key),
                    "{key}: {language:?}"
                );
            }
        }
    }

    #[test]
    fn substitutions_never_translate_or_expand_user_content() {
        activate(Language::Chinese);
        assert_eq!(
            render(
                "Conserver « {0} » ?",
                &[("0", "Enregistrer {0} Grüße 中文".into())]
            ),
            "保存“Enregistrer {0} Grüße 中文”？"
        );
        activate(Language::French);
    }
}
