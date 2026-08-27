use std::collections::HashMap;

use crate::resource_pack::ResourcePack;

pub const LOCALES: [(&str, &str); 16] = [
    ("de-de", "Deutsch"),
    ("en-gb", "English (UK)"),
    ("en-us", "English (US)"),
    ("es-es", "Español"),
    ("fr-fr", "Français"),
    ("hu-hu", "Magyar"),
    ("id-id", "Bahasa Indonesia"),
    ("it-it", "Italiano"),
    ("nb-no", "Norsk Bokmål"),
    ("nl-nl", "Nederlands"),
    ("nn-no", "Norsk Nynorsk"),
    ("pl-pl", "Polski"),
    ("pt-pt", "Português"),
    ("ru-ru", "Русский"),
    ("tr-tr", "Türkçe"),
    ("uk-ua", "Українська"),
];

pub struct Localization {
    selected: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

impl Localization {
    pub fn load(locale: &str, packs: &[ResourcePack]) -> Self {
        let mut fallback = parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/en-us.json"
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
            result = result.replacen("%s", argument, 1);
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

fn built_in(locale: &str) -> Option<&'static str> {
    Some(match locale.to_ascii_lowercase().as_str() {
        "de-de" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/de-de.json"
        )),
        "en-gb" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/en-gb.json"
        )),
        "en-us" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/en-us.json"
        )),
        "es-es" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/es-es.json"
        )),
        "fr-fr" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/fr-fr.json"
        )),
        "hu-hu" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/hu-hu.json"
        )),
        "id-id" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/id-id.json"
        )),
        "it-it" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/it-it.json"
        )),
        "nb-no" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/nb-no.json"
        )),
        "nl-nl" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/nl-nl.json"
        )),
        "nn-no" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/nn-no.json"
        )),
        "pl-pl" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/pl-pl.json"
        )),
        "pt-pt" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/pt-pt.json"
        )),
        "ru-ru" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/ru-ru.json"
        )),
        "tr-tr" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/tr-tr.json"
        )),
        "uk-ua" => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/assets/localization/uk-ua.json"
        )),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{LOCALES, Localization};

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
    fn substitutions_follow_java_string_format_order() {
        let localization = Localization::load("en-us", &[]);
        assert_eq!(
            localization.format("minicraft.displays.title.display.version", &["2.2.4"]),
            "Version 2.2.4"
        );
    }
}
