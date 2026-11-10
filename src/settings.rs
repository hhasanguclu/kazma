//! Persistent settings in `%LOCALAPPDATA%\kazma\config.txt` (`key = value` lines),
//! plus `kz config` to view and change them.

use crate::{oplog, tr, ui};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const KEYS: [(&str, &str); 2] = [
    ("lang", "auto | en | tr"),
    ("theme", "green | amber | white | blue"),
];

pub fn file() -> Option<PathBuf> {
    oplog::data_dir().map(|d| d.join("config.txt"))
}

pub fn load() -> BTreeMap<String, String> {
    let Some(f) = file() else {
        return BTreeMap::new();
    };
    std::fs::read_to_string(f)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let l = l.split('#').next()?.trim();
            let (k, v) = l.split_once('=')?;
            Some((k.trim().to_lowercase(), v.trim().to_lowercase()))
        })
        .collect()
}

pub fn get(key: &str) -> Option<String> {
    load().remove(key)
}

fn save(map: &BTreeMap<String, String>) -> anyhow::Result<PathBuf> {
    let f = file().ok_or_else(|| anyhow::anyhow!("no settings folder"))?;
    if let Some(d) = f.parent() {
        std::fs::create_dir_all(d)?;
    }
    let body: String = map.iter().map(|(k, v)| format!("{k} = {v}\n")).collect();
    std::fs::write(&f, format!("# kazma settings (kz config)\n{body}"))?;
    Ok(f)
}

fn valid(key: &str, value: &str) -> bool {
    match key {
        "lang" => matches!(value, "auto" | "en" | "tr"),
        "theme" => matches!(value, "green" | "amber" | "white" | "blue"),
        _ => false,
    }
}

pub fn run(key: Option<String>, value: Option<String>) -> anyhow::Result<()> {
    match (key, value) {
        (Some(k), Some(v)) => {
            let (k, v) = (k.to_lowercase(), v.to_lowercase());
            if !valid(&k, &v) {
                let hint = KEYS
                    .iter()
                    .find(|(n, _)| *n == k)
                    .map(|(_, h)| *h)
                    .unwrap_or("lang, theme");
                anyhow::bail!(
                    "{}",
                    tr!("invalid value. {k}: {hint}", "geçersiz değer. {k}: {hint}")
                );
            }
            let mut m = load();
            m.insert(k.clone(), v.clone());
            let f = save(&m)?;
            ui::ok(&tr!(
                "{k} = {v}  (saved to {})",
                "{k} = {v}  ({} dosyasına kaydedildi)",
                f.display()
            ));
        }
        (Some(k), None) => println!(
            "{}",
            get(&k.to_lowercase()).unwrap_or_else(|| "auto".into())
        ),
        _ => {
            ui::print_banner(&tr!("config :: settings", "config :: ayarlar"));
            let m = load();
            for (k, hint) in KEYS {
                let v = m.get(k).cloned().unwrap_or_else(|| "auto".into());
                println!("  {k:<8} = {v:<8}  ({hint})");
            }
            println!();
            println!(
                "  {}",
                tr!(
                    "change:  kz config lang tr    kz config theme amber",
                    "değiştir:  kz config lang en    kz config theme amber"
                )
            );
            if let Some(f) = file() {
                println!("  {}\n", f.display());
            }
        }
    }
    Ok(())
}
