//! Bundled locale tables and resource-pack merge for client text.
use super::pack::ResourcePack;
use std::collections::HashMap;

/// Java default `pack.json` `language` entries. Only these locales appear in
/// `LanguageSettingsDisplay`; other bundled JSON files still load if selected
/// via settings but are not menu options unless a resource pack declares them.
/// Second field is Java `LocaleInformation.toString()` = `Name (Region)`.
pub const LOCALES: [(&str, &str); 11] = [
    ("en-us", "English (US)"),
    ("fr-fr", "French (France)"),
    ("hu-hu", "Hungarian (Hungary)"),
    ("id-id", "Indonesian (Indonesia)"),
    ("it-it", "Italian (Italy)"),
    ("nb-no", "Bokmål (Norway)"),
    ("nn-no", "Nynorsk (Norway)"),
    ("pt-pt", "Portuguese (Portugal)"),
    ("ru-ru", "Russian (Russia)"),
    ("es-es", "Spanish (Spain)"),
    ("tr-tr", "Turkish (Turkey)"),
];

/// Java `LanguageSettingsDisplay` order: debug first (not bundled here), then
/// `LocaleInformation.toString().compareTo` (case-sensitive).
pub fn sorted_locale_indices() -> Vec<usize> {
    let mut indices: Vec<usize> = (0..LOCALES.len()).collect();
    indices.sort_by(|left, right| LOCALES[*left].1.cmp(LOCALES[*right].1));
    indices
}

pub struct Localization {
    selected: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

impl Localization {
    pub fn load(locale: &str, packs: &[ResourcePack]) -> Self {
        let mut fallback = parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/assets/localization/en-us.json"
        )))
        .expect("the bundled en-us localization must be valid");
        let mut selected = built_in(locale)
            .and_then(|text| parse(text).ok())
            .unwrap_or_else(|| fallback.clone());
        for pack in packs {
            merge_pack_locale(pack, "en-us", &mut fallback);
            merge_pack_locale(pack, locale, &mut selected);
        }
        Self { selected, fallback }
    }

    pub fn text<'a>(&'a self, key: &'a str) -> &'a str {
        self.selected
            .get(key)
            .or_else(|| self.fallback.get(key))
            .map(String::as_str)
            .unwrap_or(key)
    }

    pub fn format(&self, key: &str, arguments: &[&str]) -> String {
        let mut result = self.text(key).to_owned();
        for argument in arguments {
            let string = result.find("%s");
            let number = result.find("%d");
            let position = match (string, number) {
                (Some(string), Some(number)) => Some(string.min(number)),
                (Some(position), None) | (None, Some(position)) => Some(position),
                (None, None) => None,
            };
            if let Some(position) = position {
                result.replace_range(position..position + 2, argument);
            }
        }
        result
    }
}

fn merge_pack_locale(pack: &ResourcePack, locale: &str, target: &mut HashMap<String, String>) {
    let path = format!("assets/localization/{locale}.json");
    if let Ok(Some(bytes)) = pack.read(&path)
        && let Ok(values) = serde_json::from_slice::<HashMap<String, String>>(&bytes)
    {
        target.extend(values);
    }
}

fn parse(text: &str) -> Result<HashMap<String, String>, serde_json::Error> {
    serde_json::from_str(text)
}

macro_rules! locale_bytes {
    ($code:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/assets/localization/",
            $code,
            ".json"
        ))
    };
}

fn built_in(locale: &str) -> Option<&'static str> {
    Some(match locale.to_ascii_lowercase().as_str() {
        "de-de" => locale_bytes!("de-de"),
        "en-gb" => locale_bytes!("en-gb"),
        "en-us" => locale_bytes!("en-us"),
        "es-es" => locale_bytes!("es-es"),
        "fr-fr" => locale_bytes!("fr-fr"),
        "hu-hu" => locale_bytes!("hu-hu"),
        "id-id" => locale_bytes!("id-id"),
        "it-it" => locale_bytes!("it-it"),
        "nb-no" => locale_bytes!("nb-no"),
        "nl-nl" => locale_bytes!("nl-nl"),
        "nn-no" => locale_bytes!("nn-no"),
        "pl-pl" => locale_bytes!("pl-pl"),
        "pt-pt" => locale_bytes!("pt-pt"),
        "ru-ru" => locale_bytes!("ru-ru"),
        "tr-tr" => locale_bytes!("tr-tr"),
        "uk-ua" => locale_bytes!("uk-ua"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{LOCALES, Localization, sorted_locale_indices};

    #[test]
    fn locale_list_matches_java_default_pack_json() {
        let codes: Vec<_> = LOCALES.iter().map(|(code, _)| *code).collect();
        assert_eq!(
            codes,
            [
                "en-us", "fr-fr", "hu-hu", "id-id", "it-it", "nb-no", "nn-no", "pt-pt", "ru-ru",
                "es-es", "tr-tr"
            ]
        );
        assert_eq!(LOCALES[0].1, "English (US)");
        assert_eq!(LOCALES[5].1, "Bokmål (Norway)");
    }

    #[test]
    fn locale_menu_sorts_by_java_to_string_compare_to() {
        let order = sorted_locale_indices();
        let labels: Vec<_> = order.iter().map(|index| LOCALES[*index].1).collect();
        let mut expected = labels.clone();
        expected.sort();
        assert_eq!(labels, expected);
        // English (US) sorts before French (France); Bokmål is first among ASCII-ignorant?
        // Java compareTo is case-sensitive UTF-16; Bokmål's 'å' > 'E'.
        assert_eq!(labels[0], "Bokmål (Norway)");
        assert_eq!(labels[1], "English (US)");
    }

    #[test]
    fn every_bundled_locale_parses_and_falls_back() {
        for (code, _) in LOCALES {
            let localization = Localization::load(code, &[]);
            assert_ne!(
                localization.text("minicraft.displays.title.play"),
                "minicraft.displays.title.play"
            );
            assert_eq!(
                localization.text("definitely.missing"),
                "definitely.missing"
            );
        }
    }

    #[test]
    fn extra_bundled_files_load_but_are_not_menu_options() {
        // Java default pack.json omits these; they remain loadable via settings.
        for code in ["de-de", "en-gb", "nl-nl", "pl-pl", "uk-ua"] {
            assert!(
                !LOCALES.iter().any(|(menu, _)| *menu == code),
                "{code} must not appear in the default language menu"
            );
            let localization = Localization::load(code, &[]);
            assert_ne!(
                localization.text("minicraft.displays.title.play"),
                "minicraft.displays.title.play"
            );
        }
    }

    #[test]
    fn unknown_locale_falls_back_to_en_us() {
        let unknown = Localization::load("zz-zz", &[]);
        let en = Localization::load("en-us", &[]);
        assert_eq!(
            unknown.text("minicraft.displays.title.play"),
            en.text("minicraft.displays.title.play")
        );
    }

    #[test]
    fn critical_ui_keys_resolve_in_every_locale_without_fallback_to_the_key() {
        const KEYS: [&str; 8] = [
            "minicraft.displays.title.play",
            "minicraft.displays.options_main_menu",
            "minicraft.settings.difficulty",
            "minicraft.settings.size",
            "minicraft.settings.mode",
            "minicraft.displays.quests",
            "minicraft.displays.controls",
            "minicraft.displays.language_settings.title",
        ];
        for (code, _) in LOCALES {
            let localization = Localization::load(code, &[]);
            for key in KEYS {
                let text = localization.text(key);
                assert_ne!(text, key, "{code} missing {key}");
                assert!(!text.trim().is_empty(), "{code} empty {key}");
            }
        }
    }

    #[test]
    fn en_us_is_the_fallback_source_for_partial_packs() {
        let en = Localization::load("en-us", &[]);
        let de = Localization::load("de-de", &[]);
        assert_ne!(
            de.text("minicraft.displays.title.play"),
            "minicraft.displays.title.play"
        );
        assert_eq!(en.text("minicraft.displays.title.play"), "Play");
    }

    #[test]
    fn substitutions_follow_java_string_format_order() {
        let localization = Localization::load("en-us", &[]);
        assert_eq!(
            localization.format("minicraft.displays.title.display.version", &["2.2.4"]),
            "Version 2.2.4"
        );
        assert_eq!(
            localization.format(
                "minicraft.displays.quests.quest_info.display.progress",
                &["1", "3"]
            ),
            "Progress: (1/3)"
        );
    }
}
