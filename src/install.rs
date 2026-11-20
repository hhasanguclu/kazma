//! `kz install`: set up a new PC from a short, curated list of well-known
//! programs, installed through winget (the package manager built into Windows).

use crate::upgrade::{clean_line, winget, winget_available};
use crate::{oplog, ps, tr, ui};

/// (category key, English, Turkish)
const GROUPS: &[(&str, &str, &str)] = &[
    ("web", "Browsers", "Tarayıcılar"),
    ("chat", "Chat", "Mesajlaşma"),
    ("media", "Media", "Medya"),
    ("tools", "Tools", "Araçlar"),
    ("office", "Documents", "Belgeler"),
    ("dev", "Development", "Geliştirme"),
    ("games", "Games", "Oyun"),
];

/// (category, winget id, name shown)
pub const APPS: &[(&str, &str, &str)] = &[
    ("web", "Mozilla.Firefox", "Firefox"),
    ("web", "Google.Chrome", "Google Chrome"),
    ("web", "Brave.Brave", "Brave"),
    ("web", "Vivaldi.Vivaldi", "Vivaldi"),
    ("chat", "Discord.Discord", "Discord"),
    ("chat", "Telegram.TelegramDesktop", "Telegram"),
    ("chat", "WhatsApp.WhatsApp", "WhatsApp"),
    ("chat", "SlackTechnologies.Slack", "Slack"),
    ("chat", "Zoom.Zoom", "Zoom"),
    ("media", "VideoLAN.VLC", "VLC"),
    ("media", "Spotify.Spotify", "Spotify"),
    ("media", "OBSProject.OBSStudio", "OBS Studio"),
    ("media", "GIMP.GIMP.3", "GIMP"),
    ("media", "KDE.Krita", "Krita"),
    ("media", "Audacity.Audacity", "Audacity"),
    ("media", "HandBrake.HandBrake", "HandBrake"),
    ("tools", "7zip.7zip", "7-Zip"),
    ("tools", "voidtools.Everything", "Everything (file search)"),
    ("tools", "Microsoft.PowerToys", "PowerToys"),
    ("tools", "ShareX.ShareX", "ShareX"),
    ("tools", "Bitwarden.Bitwarden", "Bitwarden"),
    ("tools", "KeePassXCTeam.KeePassXC", "KeePassXC"),
    ("tools", "Notepad++.Notepad++", "Notepad++"),
    ("tools", "qBittorrent.qBittorrent", "qBittorrent"),
    ("tools", "AntibodySoftware.WizTree", "WizTree"),
    ("tools", "Rufus.Rufus", "Rufus"),
    ("office", "TheDocumentFoundation.LibreOffice", "LibreOffice"),
    ("office", "SumatraPDF.SumatraPDF", "SumatraPDF"),
    ("office", "Obsidian.Obsidian", "Obsidian"),
    ("dev", "Microsoft.VisualStudioCode", "Visual Studio Code"),
    ("dev", "Git.Git", "Git"),
    ("dev", "Microsoft.WindowsTerminal", "Windows Terminal"),
    ("dev", "Microsoft.PowerShell", "PowerShell 7"),
    ("dev", "Python.Python.3.13", "Python 3"),
    ("dev", "OpenJS.NodeJS.LTS", "Node.js LTS"),
    ("dev", "Docker.DockerDesktop", "Docker Desktop"),
    ("games", "Valve.Steam", "Steam"),
    (
        "games",
        "EpicGames.EpicGamesLauncher",
        "Epic Games Launcher",
    ),
    ("games", "GOG.Galaxy", "GOG Galaxy"),
];

/// Ids winget lists as installed (lowercase), from one `winget list`.
fn installed() -> Vec<String> {
    let Ok(out) = winget()
        .args([
            "list",
            "--accept-source-agreements",
            "--disable-interactivity",
        ])
        .output()
    else {
        return vec![];
    };
    let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
    APPS.iter()
        .map(|a| a.1.to_lowercase())
        .filter(|id| is_listed(&text, id))
        .collect()
}

/// `id` appears in winget's table as a whole word (not inside a longer id).
pub fn is_listed(table: &str, id: &str) -> bool {
    table.lines().map(clean_line).any(|l| {
        l.split_whitespace()
            .any(|w| w.trim_end_matches('…').eq_ignore_ascii_case(id))
    })
}

fn group_name(key: &str) -> &'static str {
    GROUPS
        .iter()
        .find(|g| g.0 == key)
        .map(|g| ui::t(g.1, g.2))
        .unwrap_or("")
}

fn install_one(id: &str) -> (bool, String) {
    match winget()
        .args([
            "install",
            "--id",
            id,
            "--exact",
            "--silent",
            "--source",
            "winget",
            "--accept-package-agreements",
            "--accept-source-agreements",
            "--disable-interactivity",
        ])
        .output()
    {
        Ok(o) if o.status.success() => (true, String::new()),
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout);
            let last = text
                .lines()
                .map(clean_line)
                .rfind(|l| !l.trim().is_empty())
                .unwrap_or_default();
            (false, last.trim().to_string())
        }
        Err(e) => (false, e.to_string()),
    }
}

/// `kz install [IDS...]`: pick from the list, or install the given ids / names.
pub fn run(names: Vec<String>, dry: bool, list: bool) -> anyhow::Result<()> {
    ui::print_banner(&tr!(
        "install :: programs for a new PC (winget)",
        "install :: yeni bilgisayar için programlar (winget)"
    ));
    let have: Vec<String> = if cfg!(windows) && winget_available() {
        ps::working(&tr!(
            "asking winget what is installed...",
            "winget'e yüklü programlar soruluyor..."
        ));
        let h = installed();
        println!("{:60}", "");
        h
    } else if cfg!(windows) {
        ui::warn(&tr!(
            "winget not found. Install 'App Installer' from the Microsoft Store, then try again.",
            "winget bulunamadı. Microsoft Store'dan 'Uygulama Yükleyici'yi kurup tekrar deneyin."
        ));
        return Ok(());
    } else {
        vec![]
    };
    let is_in = |id: &str| have.contains(&id.to_lowercase());

    // the list, grouped
    if list || (names.is_empty() && (dry || !ui::interactive())) || !cfg!(windows) {
        let nw = APPS.iter().map(|a| ui::width(a.2)).max().unwrap_or(0) + 2;
        for (key, _, _) in GROUPS {
            ui::section(group_name(key));
            for (_, id, name) in APPS.iter().filter(|a| a.0 == *key) {
                let line = format!("{}{id}", ui::pad(name, nw));
                if is_in(id) {
                    ui::ok(&format!("{line}  {}", tr!("(installed)", "(yüklü)")));
                } else {
                    ui::skip(&line);
                }
            }
        }
        println!(
            "\n  {}\n",
            tr!(
                "install some: kz install firefox vlc 7zip   (or just kz install to pick)",
                "kurmak için: kz install firefox vlc 7zip   (ya da seçmek için yalnızca kz install)"
            )
        );
        if !cfg!(windows) {
            ui::warn(&tr!(
                "kz install works on Windows only.",
                "kz install yalnızca Windows'ta çalışır."
            ));
        }
        return Ok(());
    }

    // what to install
    let chosen: Vec<&(&str, &str, &str)> = if !names.is_empty() {
        let mut v = vec![];
        for n in &names {
            let q = n.to_lowercase();
            let hit = APPS.iter().find(|a| {
                a.1.to_lowercase() == q
                    || a.2.to_lowercase() == q
                    || a.2.to_lowercase().replace(['-', ' '], "") == q.replace(['-', ' '], "")
                    || a.1.to_lowercase().ends_with(&format!(".{q}"))
            });
            match hit {
                Some(a) => v.push(a),
                None => ui::warn(&tr!(
                    "not on the list: {n} (kz install --list; any winget id works with winget install)",
                    "listede yok: {n} (kz install --list; diğerleri için winget install)"
                )),
            }
        }
        v
    } else {
        let order: Vec<&(&str, &str, &str)> = GROUPS
            .iter()
            .flat_map(|(k, _, _)| APPS.iter().filter(move |a| a.0 == *k))
            .filter(|a| !is_in(a.1))
            .collect();
        if order.is_empty() {
            ui::ok(&tr!(
                "Everything on the list is already installed.",
                "Listedeki her şey zaten yüklü."
            ));
            return Ok(());
        }
        let gw = GROUPS
            .iter()
            .map(|g| ui::width(ui::t(g.1, g.2)))
            .max()
            .unwrap_or(10)
            + 2;
        let items = order
            .iter()
            .map(|a| ui::CheckItem {
                label: format!("{}{}", ui::pad(group_name(a.0), gw), a.2),
                detail: a.1.to_string(),
                size: 0,
                checked: false,
            })
            .collect();
        let Some(sel) = ui::checklist_opts(
            &tr!(
                "kazma install :: pick programs (installed ones are not listed)",
                "kazma install :: programları seç (yüklü olanlar listelenmez)"
            ),
            items,
            false,
        )?
        else {
            println!("  {}", tr!("cancelled.", "iptal edildi."));
            return Ok(());
        };
        order
            .into_iter()
            .zip(sel)
            .filter(|(_, on)| *on)
            .map(|(a, _)| a)
            .collect()
    };
    if chosen.is_empty() {
        println!("  {}", tr!("nothing selected.", "hiçbir şey seçilmedi."));
        return Ok(());
    }

    ui::section(
        if dry {
            tr!("Would install", "Kurulacaklar")
        } else {
            tr!("Installing", "Kuruluyor")
        }
        .as_str(),
    );
    let nw = chosen.iter().map(|a| ui::width(a.2)).max().unwrap_or(0) + 2;
    let (mut ok, mut fail) = (0, 0);
    for (_, id, name) in &chosen {
        let line = format!("{}{id}", ui::pad(name, nw));
        if is_in(id) {
            ui::skip(&tr!("{line}  (already installed)", "{line}  (zaten yüklü)"));
            continue;
        }
        if dry {
            ui::ok(&line);
            continue;
        }
        ps::working(&tr!("{line}  installing...", "{line}  kuruluyor..."));
        let (success, err) = install_one(id);
        if success {
            ok += 1;
            ui::ok(&format!("{line}                "));
            oplog::record("install", id);
        } else {
            fail += 1;
            ui::warn(&format!("{line}  {err}"));
            oplog::record("install", &format!("{id} FAILED {err}"));
        }
    }
    if !dry {
        println!(
            "\n  {}",
            tr!(
                "installed {ok}, failed {fail}",
                "kurulan {ok}, başarısız {fail}"
            )
        );
    }
    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog() {
        let mut ids = std::collections::HashSet::new();
        for (g, id, name) in APPS {
            assert!(GROUPS.iter().any(|x| x.0 == *g), "{id}");
            assert!(ids.insert(id.to_lowercase()), "duplicate {id}");
            assert!(id.contains('.') && !id.contains(' '), "{id}");
            assert!(!name.is_empty());
        }
    }

    #[test]
    fn finds_ids_in_winget_list() {
        let table = "Name      Id               Version\n-------------------------------\n\
                     Firefox   Mozilla.Firefox  130.0\nGit LFS   Git.GitLFS       3.5\n";
        assert!(is_listed(&table.to_lowercase(), "mozilla.firefox"));
        // a longer id must not count as a shorter one
        assert!(is_listed(&table.to_lowercase(), "git.gitlfs"));
        assert!(!is_listed(&table.to_lowercase(), "git.git"));
        assert!(!is_listed(&table.to_lowercase(), "valve.steam"));
    }
}
