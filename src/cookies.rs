//! Cookies to keep: `kz privacy` deletes browser cookies, except those of the sites
//! listed in `%LOCALAPPDATA%\kazma\cookies-keep.txt`. With such a list kazma
//! removes rows from the browser's cookie database instead of the whole file, so
//! you stay signed in where you want to.

use crate::fsutil::env_path;
use crate::sqlite::Db;
use crate::{oplog, tr, ui};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct CookieDb {
    pub browser: &'static str,
    pub path: PathBuf,
    pub firefox: bool,
}

/// (name, User Data folder below %LOCALAPPDATA%, process names)
pub const CHROMIUM: &[(&str, &str, &[&str])] = &[
    ("Chrome", "Google\\Chrome\\User Data", &["chrome"]),
    ("Edge", "Microsoft\\Edge\\User Data", &["msedge"]),
    (
        "Brave",
        "BraveSoftware\\Brave-Browser\\User Data",
        &["brave"],
    ),
    ("Vivaldi", "Vivaldi\\User Data", &["vivaldi"]),
];

fn chromium_profiles(user_data: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(user_data)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.join("Preferences").is_file())
                .collect()
        })
        .unwrap_or_default()
}

/// Cookie databases of one browser ("Firefox" or a name from CHROMIUM).
pub fn databases_of(browser: &str) -> Vec<CookieDb> {
    let mut out = vec![];
    if browser == "Firefox" {
        let profiles = env_path("APPDATA")
            .map(|r| r.join("Mozilla").join("Firefox").join("Profiles"))
            .and_then(|p| std::fs::read_dir(p).ok())
            .map(|rd| rd.flatten().map(|e| e.path()).collect::<Vec<_>>())
            .unwrap_or_default();
        for p in profiles {
            let f = p.join("cookies.sqlite");
            if f.is_file() {
                out.push(CookieDb {
                    browser: "Firefox",
                    path: f,
                    firefox: true,
                });
            }
        }
        return out;
    }
    let Some((name, dir, _)) = CHROMIUM.iter().find(|b| b.0 == browser) else {
        return out;
    };
    let Some(ud) = env_path("LOCALAPPDATA").map(|l| l.join(dir)) else {
        return out;
    };
    for prof in chromium_profiles(&ud) {
        // newer versions keep it under Network\, older ones in the profile
        for f in [prof.join("Network").join("Cookies"), prof.join("Cookies")] {
            if f.is_file() {
                out.push(CookieDb {
                    browser: name,
                    path: f,
                    firefox: false,
                });
            }
        }
    }
    out
}

pub fn all_databases() -> Vec<CookieDb> {
    CHROMIUM
        .iter()
        .map(|b| b.0)
        .chain(["Firefox"])
        .flat_map(databases_of)
        .collect()
}

fn table(db: &CookieDb) -> (&'static str, &'static str) {
    if db.firefox {
        ("moz_cookies", "host")
    } else {
        ("cookies", "host_key")
    }
}

// ---- the keep list ----------------------------------------------------------------------

pub fn keep_file() -> Option<PathBuf> {
    oplog::data_dir().map(|d| d.join("cookies-keep.txt"))
}

/// "https://www.GitHub.com/x" -> "www.github.com"; "*.example.org" -> "example.org"
pub fn normalize(s: &str) -> Option<String> {
    let s = s.trim().to_lowercase();
    let s = s.split("://").last().unwrap_or("");
    let s = s.split(['/', ':', '?', '#']).next().unwrap_or("");
    let s = s
        .trim_start_matches("*.")
        .trim_start_matches('.')
        .trim_end_matches('.');
    let ok = !s.is_empty()
        && s.contains('.')
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '.' || c == '_');
    ok.then(|| s.to_string())
}

pub fn parse_keep(text: &str) -> Vec<String> {
    let mut v: Vec<String> = text
        .lines()
        .map(|l| l.split('#').next().unwrap_or(""))
        .filter_map(normalize)
        .collect();
    v.sort();
    v.dedup();
    v
}

pub fn load_keep() -> Vec<String> {
    keep_file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .map(|t| parse_keep(&t))
        .unwrap_or_default()
}

fn save_keep(list: &[String]) -> anyhow::Result<PathBuf> {
    let f = keep_file().ok_or_else(|| anyhow::anyhow!("no data folder"))?;
    if let Some(d) = f.parent() {
        std::fs::create_dir_all(d)?;
    }
    let mut list = list.to_vec();
    list.sort();
    list.dedup();
    let mut body = String::from(
        "# kazma: sites whose cookies kz privacy keeps (one per line, subdomains included)\n",
    );
    for d in &list {
        body.push_str(d);
        body.push('\n');
    }
    std::fs::write(&f, body)?;
    Ok(f)
}

/// Is a cookie for `host` (".mail.google.com") covered by the keep list?
pub fn kept(host: &str, keep: &[String]) -> bool {
    let h = host.trim_start_matches('.').to_lowercase();
    keep.iter()
        .any(|k| h == *k || h.ends_with(&format!(".{k}")))
}

/// The site a host belongs to: "accounts.google.com" -> "google.com",
/// "www.bbc.co.uk" -> "bbc.co.uk" (a short second-level label under a
/// country code counts as part of the suffix).
pub fn site_of(host: &str) -> String {
    let h = host.trim_start_matches('.').to_lowercase();
    let labels: Vec<&str> = h.split('.').collect();
    let n = labels.len();
    if n <= 2 || h.parse::<std::net::IpAddr>().is_ok() {
        return h;
    }
    let tld = labels[n - 1];
    let sld = labels[n - 2];
    let take = if tld.len() == 2
        && (sld.len() <= 3 || matches!(sld, "gov" | "edu" | "ltd" | "plc" | "nhs" | "school"))
    {
        3
    } else {
        2
    };
    labels[n.saturating_sub(take)..].join(".")
}

// ---- reading and filtering ---------------------------------------------------------------

/// Cookie count per host. Reads a copy so it works while the browser is open,
/// when the browser allows it.
pub fn hosts(db: &CookieDb) -> anyhow::Result<Vec<(String, i64)>> {
    let (t, c) = table(db);
    let sql = format!("SELECT {c}, COUNT(*) FROM {t} GROUP BY {c}");
    let tmp = std::env::temp_dir().join(format!(
        "kazma-cookies-{}-{}.db",
        std::process::id(),
        db.path.to_string_lossy().len()
    ));
    let res = match std::fs::copy(&db.path, &tmp) {
        Ok(_) => Db::open(&tmp, false).and_then(|d| d.pairs(&sql)),
        Err(_) => Db::open(&db.path, false).and_then(|d| d.pairs(&sql)),
    };
    let _ = std::fs::remove_file(&tmp);
    res
}

/// Delete every cookie not on the keep list. Returns (removed, kept).
pub fn filter(db: &CookieDb, keep: &[String], dry: bool) -> anyhow::Result<(u64, u64)> {
    let (t, c) = table(db);
    if dry {
        let all = hosts(db)?;
        let (k, r): (Vec<_>, Vec<_>) = all.iter().partition(|(h, _)| kept(h, keep));
        let sum = |v: Vec<&(String, i64)>| v.iter().map(|x| x.1 as u64).sum::<u64>();
        return Ok((sum(r), sum(k)));
    }
    let d = Db::open(&db.path, true)?;
    let all = d.pairs(&format!("SELECT {c}, COUNT(*) FROM {t} GROUP BY {c}"))?;
    let (keep_rows, drop_hosts): (Vec<_>, Vec<_>) =
        all.into_iter().partition(|(h, _)| kept(h, keep));
    let gone: Vec<String> = drop_hosts.into_iter().map(|(h, _)| h).collect();
    d.exec("BEGIN")?;
    match d.each(&format!("DELETE FROM {t} WHERE {c} = ?"), &gone) {
        Ok(n) => {
            d.exec("COMMIT")?;
            Ok((n, keep_rows.iter().map(|x| x.1 as u64).sum()))
        }
        Err(e) => {
            let _ = d.exec("ROLLBACK");
            Err(e)
        }
    }
}

// ---- kz privacy keep ---------------------------------------------------------------------

/// `kz privacy keep [add SITE.. | remove SITE..]`
pub fn command(args: Vec<String>) -> anyhow::Result<()> {
    ui::print_banner(&tr!(
        "privacy :: cookies to keep",
        "privacy :: saklanacak çerezler"
    ));
    let mut list = load_keep();
    let (verb, rest) = match args.split_first() {
        Some((v, r)) => (v.as_str(), r.to_vec()),
        None => ("", vec![]),
    };
    match verb {
        "add" | "remove" | "rm" => {
            if rest.is_empty() {
                anyhow::bail!("kz privacy keep {verb} SITE...");
            }
            for s in &rest {
                let Some(d) = normalize(s) else {
                    ui::warn(&tr!("not a site name: {s}", "site adı değil: {s}"));
                    continue;
                };
                if verb == "add" {
                    if !list.contains(&d) {
                        list.push(d.clone());
                    }
                    ui::ok(&tr!("keeping cookies of {d}", "{d} çerezleri saklanacak"));
                } else {
                    list.retain(|x| *x != d);
                    ui::ok(&tr!("no longer keeping {d}", "{d} artık saklanmayacak"));
                }
            }
            save_keep(&list)?;
            oplog::record("privacy", &format!("keep {verb} {}", rest.join(" ")));
            println!();
            Ok(())
        }
        "" if ui::interactive() && cfg!(windows) => pick(list),
        "" => {
            show(&list);
            Ok(())
        }
        other => anyhow::bail!(
            "{}",
            tr!(
                "unknown action: {other}. use kz privacy keep [add | remove] SITE",
                "bilinmeyen işlem: {other}. kz privacy keep [add | remove] SITE kullanın"
            )
        ),
    }
}

fn show(list: &[String]) {
    if list.is_empty() {
        ui::skip(&tr!(
            "no sites yet: kz privacy deletes every cookie it is asked to",
            "henüz site yok: kz privacy istenen tüm çerezleri siler"
        ));
    }
    for d in list {
        ui::ok(d);
    }
    println!(
        "\n  {}",
        tr!(
            "add: kz privacy keep add github.com   remove: kz privacy keep remove github.com",
            "ekle: kz privacy keep add github.com   çıkar: kz privacy keep remove github.com"
        )
    );
    if let Some(f) = keep_file() {
        println!("  {}\n", tr!("file: {}", "dosya: {}", f.display()));
    }
}

/// Pick the sites to keep from the cookies the browsers have now.
fn pick(list: Vec<String>) -> anyhow::Result<()> {
    crate::ps::working(&tr!("reading cookies...", "çerezler okunuyor..."));
    // site -> (cookies, browsers)
    let mut sites: BTreeMap<String, (i64, Vec<&'static str>)> = BTreeMap::new();
    let mut unreadable = vec![];
    for db in all_databases() {
        match hosts(&db) {
            Ok(h) => {
                for (host, n) in h {
                    let e = sites.entry(site_of(&host)).or_default();
                    e.0 += n;
                    if !e.1.contains(&db.browser) {
                        e.1.push(db.browser);
                    }
                }
            }
            Err(_) => {
                if !unreadable.contains(&db.browser) {
                    unreadable.push(db.browser)
                }
            }
        }
    }
    println!("{:60}", "");
    for b in &unreadable {
        ui::warn(&tr!(
            "{b}: cookies could not be read, close it and try again",
            "{b}: çerezler okunamadı, tarayıcıyı kapatıp yeniden deneyin"
        ));
    }
    // every entry of the list stays a row, also without cookies right now
    let mut rows: Vec<(String, String, bool)> = sites
        .iter()
        .map(|(site, (n, browsers))| {
            (
                site.clone(),
                tr!("{n} cookies  {}", "{n} çerez  {}", browsers.join(", ")),
                list.contains(site),
            )
        })
        .collect();
    for k in &list {
        if !rows.iter().any(|r| r.0 == *k) {
            rows.push((k.clone(), tr!("on your list", "listenizde"), true));
        }
    }
    if rows.is_empty() {
        ui::ok(&tr!("No cookies found.", "Çerez bulunamadı."));
        return Ok(());
    }
    // kept sites first, then by cookie count
    rows.sort_by_key(|r| {
        (
            !r.2,
            std::cmp::Reverse(sites.get(&r.0).map(|s| s.0).unwrap_or(0)),
        )
    });
    let items = rows
        .iter()
        .map(|(site, detail, on)| ui::CheckItem {
            label: site.clone(),
            detail: detail.clone(),
            size: 0,
            checked: *on,
        })
        .collect();
    let Some(sel) = ui::checklist_opts(
        &tr!(
            "kazma privacy :: [x] = keep these cookies",
            "kazma privacy :: [x] = bu çerezleri sakla"
        ),
        items,
        false,
    )?
    else {
        println!("  {}", tr!("cancelled.", "iptal edildi."));
        return Ok(());
    };
    let new: Vec<String> = rows
        .iter()
        .zip(sel)
        .filter(|(_, on)| *on)
        .map(|(r, _)| r.0.clone())
        .collect();
    let f = save_keep(&new)?;
    oplog::record("privacy", &format!("keep {} sites", new.len()));
    ui::ok(&tr!(
        "keeping cookies of {} sites",
        "{} sitenin çerezleri saklanacak",
        new.len()
    ));
    println!("  {}\n", tr!("file: {}", "dosya: {}", f.display()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(
            normalize("https://www.GitHub.com/x").as_deref(),
            Some("www.github.com")
        );
        assert_eq!(normalize("*.example.org").as_deref(), Some("example.org"));
        assert_eq!(normalize(".google.com").as_deref(), Some("google.com"));
        assert_eq!(normalize("localhost"), None);
        assert_eq!(normalize("a b.com"), None);
        assert_eq!(
            parse_keep("# x\ngithub.com\nGITHUB.com # again\n\nbad\n"),
            vec!["github.com"]
        );
    }

    #[test]
    fn matching() {
        let keep = vec!["google.com".to_string()];
        assert!(kept(".google.com", &keep));
        assert!(kept("accounts.google.com", &keep));
        assert!(!kept("notgoogle.com", &keep));
        assert!(!kept("google.com.evil.net", &keep));
    }

    #[test]
    fn sites() {
        assert_eq!(site_of(".accounts.google.com"), "google.com");
        assert_eq!(site_of("www.bbc.co.uk"), "bbc.co.uk");
        assert_eq!(site_of("www.sahibinden.com.tr"), "sahibinden.com.tr");
        assert_eq!(site_of("github.com"), "github.com");
        assert_eq!(site_of("192.168.1.10"), "192.168.1.10");
        assert_eq!(site_of("a.b.example.de"), "example.de");
    }

    #[cfg(windows)]
    #[test]
    fn filters_a_cookie_database() {
        let p = std::env::temp_dir().join(format!("kazma-ck-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&p);
        std::fs::write(&p, b"").unwrap();
        {
            let d = Db::open(&p, true).unwrap();
            d.exec("CREATE TABLE cookies (host_key TEXT, name TEXT)")
                .unwrap();
            d.exec(
                "INSERT INTO cookies VALUES ('.github.com','a'),('github.com','b'),\
                 ('.ads.example.net','c'),('mail.google.com','d')",
            )
            .unwrap();
        }
        let db = CookieDb {
            browser: "Chrome",
            path: p.clone(),
            firefox: false,
        };
        let keep = vec!["github.com".to_string()];
        assert_eq!(filter(&db, &keep, true).unwrap(), (2, 2));
        assert_eq!(filter(&db, &keep, false).unwrap(), (2, 2));
        let left = hosts(&db).unwrap();
        assert!(left.iter().all(|(h, _)| kept(h, &keep)), "{left:?}");
        let _ = std::fs::remove_file(&p);
    }
}
