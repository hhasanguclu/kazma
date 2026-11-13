//! Languages. English and Turkish live in the code (`tr!` / `ui::t`); the other
//! languages are JSON files in locales/ that map the English text to a
//! translation. A missing entry falls back to English.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    Tr,
    ZhHans,
    ZhHant,
    Ja,
    Ko,
    De,
    Fr,
    Pl,
    Uk,
    Ru,
}

/// (language, code, name in that language, embedded translations)
pub const LANGS: &[(Lang, &str, &str, &str)] = &[
    (Lang::En, "en", "English", ""),
    (Lang::Tr, "tr", "Türkçe", ""),
    (
        Lang::ZhHans,
        "zh-CN",
        "简体中文",
        include_str!("../locales/zh-CN.json"),
    ),
    (
        Lang::ZhHant,
        "zh-TW",
        "繁體中文",
        include_str!("../locales/zh-TW.json"),
    ),
    (Lang::Ja, "ja", "日本語", include_str!("../locales/ja.json")),
    (Lang::Ko, "ko", "한국어", include_str!("../locales/ko.json")),
    (
        Lang::De,
        "de",
        "Deutsch",
        include_str!("../locales/de.json"),
    ),
    (
        Lang::Fr,
        "fr",
        "Français",
        include_str!("../locales/fr.json"),
    ),
    (Lang::Pl, "pl", "Polski", include_str!("../locales/pl.json")),
    (
        Lang::Uk,
        "uk",
        "Українська",
        include_str!("../locales/uk.json"),
    ),
    (
        Lang::Ru,
        "ru",
        "Русский",
        include_str!("../locales/ru.json"),
    ),
];

/// Index into LANGS; NONE until the first use picks one.
static LANG: AtomicU8 = AtomicU8::new(NONE);
const NONE: u8 = u8::MAX;
/// One parsed table per language, filled on first use.
static TABLES: [OnceLock<HashMap<&'static str, &'static str>>; 11] =
    [const { OnceLock::new() }; 11];

fn index(l: Lang) -> usize {
    LANGS.iter().position(|x| x.0 == l).unwrap_or(0)
}

pub fn lang() -> Lang {
    let i = LANG.load(Ordering::Relaxed);
    if i != NONE {
        return LANGS[i as usize].0;
    }
    let l = detect();
    LANG.store(index(l) as u8, Ordering::Relaxed);
    l
}

/// Switch language while kazma runs (`kz config`).
pub fn set(l: Lang) {
    LANG.store(index(l) as u8, Ordering::Relaxed);
}

/// What `auto` means here: the system language, or English.
pub fn auto() -> Lang {
    system().unwrap_or(Lang::En)
}

/// Pick the language again from KAZMA_LANG / config / system.
pub fn reload() {
    set(detect());
}

pub fn code() -> &'static str {
    LANGS.iter().find(|l| l.0 == lang()).map_or("en", |l| l.1)
}

/// "de", "zh-TW", "zh_Hant_HK", "pt-BR" ... -> a supported language, if any.
pub fn parse(s: &str) -> Option<Lang> {
    let s = s.trim().to_lowercase().replace('_', "-");
    let s = s.split('.').next().unwrap_or("");
    let mut parts = s.split('-');
    let main = parts.next().unwrap_or("");
    let rest: Vec<&str> = parts.collect();
    Some(match main {
        "en" => Lang::En,
        "tr" => Lang::Tr,
        "zh" if rest
            .iter()
            .any(|p| matches!(*p, "tw" | "hk" | "mo" | "hant")) =>
        {
            Lang::ZhHant
        }
        "zh" => Lang::ZhHans,
        "ja" => Lang::Ja,
        "ko" => Lang::Ko,
        "de" => Lang::De,
        "fr" => Lang::Fr,
        "pl" => Lang::Pl,
        "uk" => Lang::Uk,
        "ru" => Lang::Ru,
        _ => return None,
    })
}

/// KAZMA_LANG, then `kz config lang`, then the system display language, then English.
fn detect() -> Lang {
    let pick = std::env::var("KAZMA_LANG")
        .ok()
        .or_else(|| crate::settings::get("lang"))
        .filter(|v| !v.is_empty() && v != "auto");
    pick.as_deref()
        .and_then(parse)
        .or_else(system)
        .unwrap_or(Lang::En)
}

/// The Windows display language (LANGID), or LANG / LC_ALL elsewhere.
#[cfg(windows)]
pub fn system() -> Option<Lang> {
    let id = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() };
    from_langid(id)
}

#[cfg(not(windows))]
pub fn system() -> Option<Lang> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .find(|v| !v.is_empty() && v != "C" && v != "POSIX")
        .and_then(|v| parse(&v))
}

#[cfg_attr(not(windows), allow(dead_code))]
/// Windows LANGID: low 10 bits = primary language, high 6 = sublanguage.
pub fn from_langid(id: u16) -> Option<Lang> {
    let (primary, sub) = (id & 0x3ff, id >> 10);
    Some(match primary {
        0x09 => Lang::En,
        0x1f => Lang::Tr,
        // Taiwan, Hong Kong, Macau use Traditional characters
        0x04 if matches!(sub, 0x01 | 0x03 | 0x05) => Lang::ZhHant,
        0x04 => Lang::ZhHans,
        0x11 => Lang::Ja,
        0x12 => Lang::Ko,
        0x07 => Lang::De,
        0x0c => Lang::Fr,
        0x15 => Lang::Pl,
        0x22 => Lang::Uk,
        0x19 => Lang::Ru,
        _ => return None,
    })
}

fn table() -> &'static HashMap<&'static str, &'static str> {
    let i = index(lang());
    TABLES[i].get_or_init(|| parse_table(LANGS[i].3))
}

fn parse_table(json: &str) -> HashMap<&'static str, &'static str> {
    let map: HashMap<String, String> = serde_json::from_str(json).unwrap_or_default();
    map.into_iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| (&*k.leak(), &*v.leak()))
        .collect()
}

/// The current language's text for this English text.
pub fn get(en: &str) -> Option<&'static str> {
    table().get(en).copied()
}

/// Translate a template and fill its holes with values already formatted with
/// the English spec. Keys are the holes as written in English: a name, an index,
/// or "" for each `{}` in order. None (use English) if anything does not fit.
pub fn render(en: &str, values: &[(&str, String)]) -> Option<String> {
    fill(get(en)?, values)
}

pub fn fill(template: &str, values: &[(&str, String)]) -> Option<String> {
    let mut unnamed = values.iter().filter(|(k, _)| k.is_empty());
    let mut out = String::with_capacity(template.len() + 16);
    let mut it = template.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '{' if it.peek() == Some(&'{') => {
                it.next();
                out.push('{');
            }
            '}' if it.peek() == Some(&'}') => {
                it.next();
                out.push('}');
            }
            '{' => {
                let mut inner = String::new();
                loop {
                    match it.next()? {
                        '}' => break,
                        c => inner.push(c),
                    }
                }
                let arg = inner.split(':').next().unwrap_or("").trim();
                let v = if arg.is_empty() {
                    &unnamed.next()?.1
                } else {
                    &values.iter().find(|(k, _)| *k == arg)?.1
                };
                out.push_str(v);
            }
            '}' => return None,
            c => out.push(c),
        }
    }
    Some(out)
}

#[cfg(test)]
/// The holes of a format string, as written ("" for `{}`), in order.
pub fn holes(s: &str) -> Option<Vec<String>> {
    let mut out = vec![];
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '{' if it.peek() == Some(&'{') => {
                it.next();
            }
            '}' if it.peek() == Some(&'}') => {
                it.next();
            }
            '{' => {
                let mut inner = String::new();
                loop {
                    match it.next()? {
                        '}' => break,
                        c => inner.push(c),
                    }
                }
                out.push(inner.split(':').next().unwrap_or("").trim().to_string());
            }
            '}' => return None,
            _ => {}
        }
    }
    Some(out)
}

/// A JSON object {english: translation} for the given English strings, for
/// PowerShell scripts that print text (see health.rs).
pub fn ps_table(keys: &[&str]) -> String {
    let map: HashMap<&str, &str> = keys.iter().filter_map(|k| Some((*k, get(k)?))).collect();
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".into())
}

/// The English strings a PowerShell script passes to its `T 'en' 'tr'` helper.
pub fn ps_keys(script: &str) -> Vec<String> {
    let mut out = vec![];
    let mut rest = script;
    while let Some(i) = rest.find("T '") {
        let before = rest[..i].chars().last();
        rest = &rest[i + 3..];
        if before.is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '$') {
            continue;
        }
        let mut s = String::new();
        let mut it = rest.char_indices().peekable();
        let mut end = rest.len();
        while let Some((j, c)) = it.next() {
            if c == '\'' {
                if it.peek().map(|x| x.1) == Some('\'') {
                    it.next();
                    s.push('\'');
                    continue;
                }
                end = j + 1;
                break;
            }
            s.push(c);
        }
        rest = &rest[end..];
        if !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_language_codes() {
        assert_eq!(parse("de_DE.UTF-8"), Some(Lang::De));
        assert_eq!(parse("zh-TW"), Some(Lang::ZhHant));
        assert_eq!(parse("zh_Hant_HK"), Some(Lang::ZhHant));
        assert_eq!(parse("zh-CN"), Some(Lang::ZhHans));
        assert_eq!(parse("tr"), Some(Lang::Tr));
        assert_eq!(parse("pt-BR"), None);
        assert_eq!(from_langid(0x0404), Some(Lang::ZhHant));
        assert_eq!(from_langid(0x0804), Some(Lang::ZhHans));
        assert_eq!(from_langid(0x0419), Some(Lang::Ru));
        assert_eq!(from_langid(0x041f), Some(Lang::Tr));
        assert_eq!(from_langid(0x0416), None);
    }

    #[test]
    fn fills_templates() {
        let v = [
            ("", "3".to_string()),
            ("n", "7".to_string()),
            ("", "x".to_string()),
        ];
        assert_eq!(
            fill("{n} / {} / {:>5} {{ok}}", &v).as_deref(),
            Some("7 / 3 / x {ok}")
        );
        assert_eq!(fill("{missing}", &v), None);
        assert_eq!(
            holes("{a:>4} {} {{x}} {0}"),
            Some(vec!["a".into(), "".into(), "0".into()])
        );
    }

    #[test]
    fn finds_powershell_strings() {
        let k = ps_keys("Add $S (T 'Uptime' 'Açık') (T 'it''s' 'x') $x; $q = LastT 'no' 'y'");
        assert_eq!(k, vec!["Uptime", "it's"]);
    }

    /// Every translation keeps the English text's placeholders, and every file parses.
    #[test]
    fn translations_are_consistent() {
        for (_, code, _, json) in LANGS.iter().filter(|l| !l.3.is_empty()) {
            let map: HashMap<String, String> =
                serde_json::from_str(json).unwrap_or_else(|e| panic!("{code}.json: {e}"));
            let source: Vec<serde_json::Value> =
                serde_json::from_str(include_str!("../locales/source.json")).unwrap();
            for s in &source {
                let en = s["en"].as_str().unwrap();
                let fmt = s["kind"].as_str() != Some("plain");
                let Some(t) = map.get(en).filter(|t| !t.is_empty()) else {
                    continue;
                };
                if fmt {
                    let mut a = holes(en).unwrap_or_default();
                    let mut b = holes(t).unwrap_or_else(|| panic!("{code}: bad braces in {t:?}"));
                    a.sort();
                    b.sort();
                    assert_eq!(
                        a, b,
                        "{code}: placeholders differ\n  en: {en}\n  {code}: {t}"
                    );
                }
            }
        }
    }
}
