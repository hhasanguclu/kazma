//! `kz power`: power plans, battery report, what wakes the PC.

use crate::fsutil::home;
use crate::{oplog, ps, sys, tr, ui};

/// (key, Windows plan GUID, english, turkish)
pub const PLANS: &[(&str, &str, &str, &str)] = &[
    (
        "saver",
        "a1841308-3541-4fab-bc81-f71556f20b4a",
        "Power saver",
        "Güç tasarrufu",
    ),
    (
        "balanced",
        "381b4222-f694-41f0-9685-ff5bb260df2e",
        "Balanced",
        "Dengeli",
    ),
    (
        "high",
        "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c",
        "High performance",
        "Yüksek performans",
    ),
    (
        "ultimate",
        "e9a42b02-d5df-448d-aa00-03f14749eb61",
        "Ultimate performance",
        "Nihai performans",
    ),
];

#[derive(Debug, PartialEq)]
pub struct Plan {
    pub guid: String,
    pub name: String,
    pub active: bool,
}

fn is_guid(s: &str) -> bool {
    s.len() == 36
        && s.chars().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == '-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

/// Parse `powercfg /list`. Lines look like `Power Scheme GUID: <guid>  (Name) *`
/// in any language, so only the GUID, the parenthesised name and `*` are used.
pub fn parse_list(text: &str) -> Vec<Plan> {
    text.lines()
        .filter_map(|l| {
            let guid = l.split_whitespace().find(|w| is_guid(w))?.to_lowercase();
            let name = l
                .split_once('(')
                .and_then(|(_, r)| r.rsplit_once(')'))
                .map(|(n, _)| n.trim().to_string())
                .unwrap_or_default();
            Some(Plan {
                guid,
                name,
                active: l.trim_end().ends_with('*'),
            })
        })
        .collect()
}

fn plans() -> Vec<Plan> {
    parse_list(&ps::exec("powercfg", &["/list"]).1)
}

fn show() {
    let list = plans();
    ui::section(ui::t("Power plans", "Güç planları"));
    for p in &list {
        let key = PLANS
            .iter()
            .find(|(_, g, _, _)| *g == p.guid)
            .map(|(k, _, _, _)| *k)
            .unwrap_or("");
        let line = format!("{} {:<10} {}", ui::pad(&p.name, 28), key, p.guid);
        if p.active {
            ui::ok(&format!("{line}  <- {}", ui::t("active", "etkin")))
        } else {
            println!("       {line}")
        }
    }
    println!(
        "\n  {}",
        ui::t(
            "switch:  kz power set saver | balanced | high | ultimate",
            "değiştir:  kz power set saver | balanced | high | ultimate"
        )
    );
    println!(
        "  {}\n",
        ui::t(
            "more:    kz power battery | kz power wake | kz power hibernate off|reduced|full",
            "diğer:    kz power battery | kz power wake | kz power hibernate off|reduced|full"
        )
    );
}

fn set(key: &str) -> anyhow::Result<()> {
    let (_, guid, en, trn) = PLANS.iter().find(|(k, _, _, _)| *k == key).ok_or_else(|| {
        anyhow::anyhow!("{}", tr!("unknown plan: {key}", "bilinmeyen plan: {key}"))
    })?;
    let name = ui::t(en, trn);
    // Plans hidden on this PC (e.g. Ultimate) must be copied from the template first.
    let mut target = guid.to_string();
    if !plans().iter().any(|p| p.guid == *guid) {
        let (_, out) = ps::exec("powercfg", &["-duplicatescheme", guid]);
        match out.split_whitespace().find(|w| is_guid(w)) {
            Some(g) => target = g.to_lowercase(),
            None => anyhow::bail!(
                "{}",
                tr!(
                    "{name} is not available on this PC",
                    "{name} bu bilgisayarda kullanılamıyor"
                )
            ),
        }
    }
    let (ok, out) = ps::exec("powercfg", &["/setactive", &target]);
    if !ok {
        anyhow::bail!("{}", ps::last_line(&out));
    }
    oplog::record("power", &format!("plan {key}"));
    ui::ok(&tr!("power plan: {name}", "güç planı: {name}"));
    Ok(())
}

/// off: delete hiberfil.sys (also disables Fast Startup); reduced: half size,
/// Fast Startup keeps working; full / on: normal hibernation.
fn hibernate(mode: &str) -> anyhow::Result<()> {
    let args: &[&str] = match mode {
        "off" => &["/h", "off"],
        "on" => &["/h", "on"],
        "reduced" => &["/h", "/type", "reduced"],
        "full" => &["/h", "/type", "full"],
        _ => anyhow::bail!(
            "{}",
            ui::t(
                "use: kz power hibernate off | reduced | full | on",
                "kullanım: kz power hibernate off | reduced | full | on"
            )
        ),
    };
    if !sys::is_admin() {
        anyhow::bail!(
            "{}",
            ui::t(
                "this needs an elevated terminal",
                "bunun için yönetici terminali gerekir"
            )
        );
    }
    let (ok, out) = ps::exec("powercfg", args);
    if !ok {
        anyhow::bail!("{}", ps::last_line(&out));
    }
    oplog::record("power", &format!("hibernate {mode}"));
    let msg = match mode {
        "off" => ui::t(
            "hibernation off: hiberfil.sys removed (Fast Startup is off too)",
            "hazırda bekletme kapalı: hiberfil.sys silindi (Hızlı Başlangıç da kapandı)",
        ),
        "reduced" => ui::t(
            "hibernation file reduced; Fast Startup still works",
            "hazırda bekletme dosyası küçültüldü; Hızlı Başlangıç çalışıyor",
        ),
        _ => ui::t("hibernation on", "hazırda bekletme açık"),
    };
    ui::ok(msg);
    Ok(())
}

fn battery() -> anyhow::Result<()> {
    let out = home()
        .map(|h| h.join("Desktop"))
        .filter(|d| d.is_dir())
        .unwrap_or_else(std::env::temp_dir)
        .join("battery-report.html");
    let (ok, text) = ps::exec(
        "powercfg",
        &["/batteryreport", "/output", &out.to_string_lossy()],
    );
    if !ok || !out.exists() {
        anyhow::bail!(
            "{}",
            tr!(
                "no battery report ({})",
                "pil raporu yok ({})",
                ps::last_line(&text)
            )
        );
    }
    ui::ok(&tr!("battery report: {}", "pil raporu: {}", out.display()));
    sys::open(&out);
    Ok(())
}

fn wake() {
    for (title, args) in [
        (
            ui::t("Last wake source", "Son uyanma nedeni"),
            vec!["/lastwake"],
        ),
        (
            ui::t("Wake timers", "Uyandırma zamanlayıcıları"),
            vec!["/waketimers"],
        ),
        (
            ui::t(
                "Devices allowed to wake the PC",
                "Bilgisayarı uyandırabilen aygıtlar",
            ),
            vec!["/devicequery", "wake_armed"],
        ),
    ] {
        ui::section(title);
        let (_, out) = ps::exec("powercfg", &args);
        for l in out.lines().filter(|l| !l.trim().is_empty()) {
            println!("  {}", l.trim_end());
        }
    }
    println!();
}

pub fn run(action: Option<String>, arg: Option<String>) -> anyhow::Result<()> {
    ui::print_banner(ui::t(
        "power :: plans, battery, wake",
        "power :: güç planı, pil, uyanma",
    ));
    if !cfg!(windows) {
        ui::warn(ui::t(
            "kz power works on Windows only.",
            "kz power yalnızca Windows'ta çalışır.",
        ));
        return Ok(());
    }
    match (action.as_deref(), arg.as_deref()) {
        (None, _) => {
            show();
            Ok(())
        }
        (Some("set"), Some(k)) => set(&k.to_lowercase()),
        (Some("battery"), _) => battery(),
        (Some("hibernate"), Some(mode)) => hibernate(&mode.to_lowercase()),
        (Some("wake"), _) => {
            wake();
            Ok(())
        }
        _ => anyhow::bail!(
            "{}",
            ui::t(
                "usage: kz power [set PLAN | battery | wake | hibernate MODE]",
                "kullanım: kz power [set PLAN | battery | wake | hibernate MOD]"
            )
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_powercfg_in_any_language() {
        let en = "Existing Power Schemes (* Active)\n-----------------------------------\nPower Scheme GUID: 381b4222-f694-41f0-9685-ff5bb260df2e  (Balanced) *\nPower Scheme GUID: 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c  (High performance)\n";
        let tr = "Varolan Güç Düzenleri (* Etkin)\nGüç Düzeni GUID'si: 381b4222-f694-41f0-9685-ff5bb260df2e  (Dengeli)\nGüç Düzeni GUID'si: a1841308-3541-4fab-bc81-f71556f20b4a  (Güç tasarrufu) *\n";
        let p = parse_list(en);
        assert_eq!(p.len(), 2);
        assert!(p[0].active && p[0].name == "Balanced");
        assert!(!p[1].active);
        let p = parse_list(tr);
        assert_eq!(p[1].name, "Güç tasarrufu");
        assert!(p[1].active);
        assert!(PLANS.iter().all(|(_, g, _, _)| is_guid(g)));
    }
}
