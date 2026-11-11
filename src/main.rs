//! kazma: an oldschool, all-ASCII system cleaner and disk explorer for Windows.

mod analyze;
mod backup;
mod bench;
mod clean;
mod context;
mod debloat;
mod drivers;
mod dupes;
mod empty;
mod fsutil;
mod health;
mod installer;
mod net;
mod oplog;
mod optimize;
mod ports;
mod power;
mod privacy;
mod ps;
mod purge;
mod remove;
mod report;
mod restore;
mod schedule;
mod settings;
mod startup;
mod status;
mod sys;
mod ui;
mod uninstall;
mod update;
mod upgrade;
mod whitelist;

use clap::{Parser, Subcommand};
use crossterm::event::{self, KeyCode};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "kz",
    version,
    about = "Oldschool ASCII system cleaner & disk explorer for Windows"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Remove caches, temp files, logs and empty the Recycle Bin
    Clean {
        /// Show what would be removed without deleting anything
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Skip the selection screen and clean every item
        #[arg(long, short)]
        yes: bool,
    },
    /// Interactive disk usage explorer
    #[command(alias = "analyse")]
    Analyze { path: Option<PathBuf> },
    /// Live system dashboard (CPU, memory, disks, network, processes)
    Status,
    /// Remove project build artifacts (node_modules, target, bin/obj, .venv...)
    Purge {
        /// Folders to scan (default: common project folders in your profile)
        paths: Vec<PathBuf>,
        #[arg(long, short = 'n')]
        dry_run: bool,
        #[arg(long, short)]
        yes: bool,
    },
    /// Update installed programs with winget
    Upgrade {
        /// List available updates without installing
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Upgrade everything without the selection screen
        #[arg(long, short)]
        yes: bool,
    },
    /// Find and install driver updates from Windows Update
    Drivers {
        /// List installed third-party drivers by age instead
        #[arg(long)]
        installed: bool,
        #[arg(long, short = 'n')]
        dry_run: bool,
        #[arg(long, short)]
        yes: bool,
        /// Do not create a system restore point before installing
        #[arg(long)]
        no_restore_point: bool,
    },
    /// Find leftover installer files (setup .exe, .msi, .iso) in Downloads and on the Desktop
    Installer {
        /// Folders to scan instead of Downloads / Desktop
        paths: Vec<PathBuf>,
        #[arg(long, short = 'n')]
        dry_run: bool,
        #[arg(long, short)]
        yes: bool,
    },
    /// Uninstall programs and clean up their leftovers
    Uninstall {
        /// Only list installed programs
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Use the program's silent uninstaller when it has one
        #[arg(long)]
        quiet: bool,
    },
    /// Windows maintenance: DNS flush, icon cache, SSD TRIM, DISM, SFC
    Optimize {
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Run the default tasks without the selection screen
        #[arg(long, short)]
        yes: bool,
    },
    /// Show or edit paths that kazma must never delete
    Whitelist {
        /// Protect a path (can be repeated)
        #[arg(long, value_name = "PATH")]
        add: Vec<String>,
        /// Stop protecting a path (can be repeated)
        #[arg(long, value_name = "PATH")]
        remove: Vec<String>,
        /// Open the whitelist file in Notepad
        #[arg(long)]
        edit: bool,
    },
    /// Programs that start with Windows: turn them on or off
    Startup {
        /// Only list, do not open the selection screen
        #[arg(long, short)]
        list: bool,
    },
    /// System health report: disks, battery, crashes, updates, security
    Health,
    /// Find duplicate files (sent to the Recycle Bin)
    Dupes {
        /// Folders to scan (default: Documents, Downloads, Desktop, Pictures, Videos, Music)
        paths: Vec<PathBuf>,
        /// Ignore files smaller than this many MB
        #[arg(long, default_value_t = 1)]
        min_mb: u64,
        #[arg(long, short = 'n')]
        dry_run: bool,
    },
    /// Remove preinstalled promo apps and turn off Windows ads
    Debloat {
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Turn the ad / suggestion settings back on
        #[arg(long)]
        restore: bool,
    },
    /// Clear usage traces: recent files, Run history, browser history / cookies
    Privacy {
        #[arg(long, short = 'n')]
        dry_run: bool,
        #[arg(long, short)]
        yes: bool,
    },
    /// Automatic cleaning: daily | weekly | monthly | off | status
    Schedule {
        frequency: Option<String>,
        /// Time of day, HH:MM
        #[arg(long, default_value = "11:00")]
        time: String,
    },
    /// Update kazma itself from GitHub
    Update {
        /// Only check, do not install
        #[arg(long)]
        check: bool,
    },
    /// Show or change settings: kz config lang tr | kz config theme amber
    Config {
        key: Option<String>,
        value: Option<String>,
    },
    /// Backup for a new PC: app list, drivers, program list, kazma settings.
    /// `kz backup [FOLDER]` or `kz backup restore FOLDER`
    Backup {
        /// "restore", or the backup folder
        action: Option<String>,
        /// folder to restore from
        folder: Option<PathBuf>,
        #[arg(long, short = 'n')]
        dry_run: bool,
    },
    /// Which program listens on which port; --kill PORT closes it
    Ports {
        /// Filter by port number or program name
        filter: Option<String>,
        #[arg(long, value_name = "PORT")]
        kill: Option<u16>,
    },
    /// Network: info | dns | dns set NAME | speed | wifi [--show]
    Net {
        action: Option<String>,
        arg: Option<String>,
        arg2: Option<String>,
        /// Show Wi-Fi passwords
        #[arg(long)]
        show: bool,
    },
    /// Empty folders, broken shortcuts and 0-byte files
    Empty {
        paths: Vec<PathBuf>,
        #[arg(long, short = 'n')]
        dry_run: bool,
    },
    /// Power plans: kz power [set PLAN | battery | wake]
    Power {
        action: Option<String>,
        arg: Option<String>,
    },
    /// Write a system report (HTML) to the Desktop
    Report {
        /// Output file
        #[arg(long, short)]
        output: Option<PathBuf>,
        /// Do not open it in the browser
        #[arg(long)]
        no_open: bool,
    },
    /// System Restore: list | create [DESC] | enable | open
    Restore {
        action: Option<String>,
        description: Option<String>,
    },
    /// Explorer right-click "Analyze with kazma": install | uninstall | status
    Context { action: Option<String> },
    /// Disk speed test (sequential and 4K random)
    Bench {
        /// Folder on the drive to test (default: temp folder)
        path: Option<PathBuf>,
        #[arg(long, default_value_t = 1024)]
        size_mb: u64,
    },
    /// Your own clean rules (extra folders for `kz clean`); --edit opens Notepad
    Rules {
        #[arg(long)]
        edit: bool,
    },
    /// Uninstall kazma from this computer (exe, PATH entry, log)
    Remove {
        #[arg(long, short = 'n')]
        dry_run: bool,
        #[arg(long, short)]
        yes: bool,
    },
    /// Show the log of past cleanup operations
    History {
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
}

/// Menu rows: a group header (`key == ' '`) or (hotkey, command, english, turkish).
const MENU: &[(char, &str, &str, &str)] = &[
    (' ', "", "clean up", "temizlik"),
    (
        '1',
        "clean",
        "caches, temp files, logs, recycle bin",
        "önbellek, geçici dosya, log, geri dönüşüm kutusu",
    ),
    (
        '2',
        "analyze",
        "explore what is using your disk",
        "diski ne kaplıyor, gez ve gör",
    ),
    (
        '3',
        "purge",
        "project build artifacts (node_modules, target...)",
        "proje derleme çıktıları (node_modules, target...)",
    ),
    (
        '4',
        "installer",
        "old setup files in Downloads / Desktop",
        "İndirilenler / Masaüstü'ndeki eski kurulum dosyaları",
    ),
    (
        '5',
        "dupes",
        "find duplicate files",
        "yinelenen dosyaları bul",
    ),
    (
        '6',
        "empty",
        "empty folders, broken shortcuts, 0-byte files",
        "boş klasör, bozuk kısayol, 0 baytlık dosya",
    ),
    (
        '7',
        "privacy",
        "recent files, run history, browser traces",
        "son dosyalar, çalıştır geçmişi, tarayıcı izleri",
    ),
    (' ', "", "programs & drivers", "programlar ve sürücüler"),
    (
        '8',
        "uninstall",
        "remove programs and their leftovers",
        "program kaldır, kalıntılarını temizle",
    ),
    (
        '9',
        "upgrade",
        "update installed programs (winget)",
        "yüklü programları güncelle (winget)",
    ),
    (
        '0',
        "drivers",
        "driver updates from windows update",
        "windows update'ten sürücü güncellemeleri",
    ),
    (
        's',
        "startup",
        "programs that start with windows",
        "windows ile başlayan programlar",
    ),
    (
        'b',
        "debloat",
        "remove promo apps, turn off ads",
        "tanıtım uygulamalarını kaldır, reklamları kapat",
    ),
    (
        'a',
        "backup",
        "move-to-a-new-PC kit: apps, drivers, settings",
        "yeni bilgisayara taşıma: programlar, sürücüler, ayarlar",
    ),
    (' ', "", "system", "sistem"),
    (
        'c',
        "health",
        "health report: disks, battery, crashes",
        "sağlık raporu: disk, pil, çökmeler",
    ),
    (
        't',
        "status",
        "live monitor and process manager",
        "canlı izleme ve süreç yöneticisi",
    ),
    (
        'o',
        "optimize",
        "dns flush, icon cache, ssd trim, dism, sfc",
        "dns, ikon önbelleği, ssd trim, dism, sfc",
    ),
    (
        'p',
        "power",
        "power plans, battery report, wake sources",
        "güç planı, pil raporu, uyandırma nedenleri",
    ),
    (
        'e',
        "restore",
        "system restore points",
        "sistem geri yükleme noktaları",
    ),
    (
        'r',
        "report",
        "system report as an HTML file",
        "HTML sistem raporu",
    ),
    ('v', "bench", "disk speed test", "disk hız testi"),
    (
        'z',
        "schedule",
        "automatic weekly cleaning",
        "otomatik haftalık temizlik",
    ),
    (' ', "", "network", "ağ"),
    (
        'i',
        "net",
        "ip, latency, dns, speed test, wifi",
        "ip, gecikme, dns, hız testi, wifi",
    ),
    (
        'x',
        "ports",
        "which program uses which port",
        "hangi port hangi programda",
    ),
    (' ', "", "kazma", "kazma"),
    (
        'w',
        "whitelist",
        "paths kazma must never delete",
        "kazma'nın asla silmeyeceği yollar",
    ),
    (
        'l',
        "rules",
        "your own clean rules",
        "kendi temizlik kuralların",
    ),
    (
        'm',
        "context",
        "right-click menu in Explorer",
        "Gezgin sağ tık menüsü",
    ),
    (
        'g',
        "config",
        "language and color theme",
        "dil ve renk teması",
    ),
    ('h', "history", "show past operations", "geçmiş işlemler"),
    ('u', "update", "update kazma itself", "kazma'yı güncelle"),
    ('q', "quit", "exit", "çıkış"),
];

fn selectable(i: usize) -> bool {
    MENU.get(i).is_some_and(|m| m.0 != ' ')
}

fn menu(
    start: usize,
    notice: &std::sync::mpsc::Receiver<String>,
    newer: &mut Option<String>,
) -> anyhow::Result<(&'static str, usize)> {
    let mut terminal = ui::tui_start();
    let mut state = ListState::default().with_selected(Some(start));
    let mut list_area = ratatui::layout::Rect::default();
    let res = (|| -> anyhow::Result<(&'static str, usize)> {
        loop {
            if let Ok(tag) = notice.try_recv() {
                *newer = Some(tag);
            }
            terminal.draw(|f| {
                let banner_h = ui::BANNER.lines().count() as u16 + 1;
                let [top, list, foot] = Layout::vertical([
                    Constraint::Length(banner_h),
                    Constraint::Min(5),
                    Constraint::Length(1),
                ])
                .areas(f.area());
                list_area = list;
                let mut lines: Vec<Line> = ui::BANNER
                    .lines()
                    .map(|l| Line::from(Span::styled(format!("  {l}"), Style::new().fg(ui::fg()))))
                    .collect();
                let mut sub = vec![Span::styled(
                    format!(
                        "  v{}  ::  {}",
                        env!("CARGO_PKG_VERSION"),
                        tr!(
                            "oldschool system toolkit for windows",
                            "windows için oldschool sistem araç kutusu"
                        )
                    ),
                    Style::new().fg(ui::dim()),
                )];
                if let Some(tag) = newer.as_ref() {
                    sub.push(Span::styled(
                        tr!(
                            "   * {tag} is out: press u *",
                            "   * {tag} çıktı: u'ya basın *"
                        ),
                        Style::new().fg(ui::hi()).add_modifier(Modifier::BOLD),
                    ));
                }
                lines.push(Line::from(sub));
                f.render_widget(Paragraph::new(lines), top);

                let tr_mode = ui::is_tr();
                let items: Vec<ListItem> = MENU
                    .iter()
                    .map(|(key, name, en, trd)| {
                        let desc = if tr_mode { *trd } else { *en };
                        if *key == ' ' {
                            ListItem::new(Line::from(Span::styled(
                                format!(" -- {desc} --"),
                                Style::new().fg(ui::dim()),
                            )))
                        } else {
                            ListItem::new(Line::from(vec![
                                Span::styled(format!("  {key}. "), Style::new().fg(ui::dim())),
                                Span::styled(
                                    format!("{name:<11}"),
                                    Style::new().fg(ui::hi()).add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(desc, Style::new().fg(ui::fg())),
                            ]))
                        }
                    })
                    .collect();
                let title = tr!("main menu", "ana menü");
                let l = List::new(items)
                    .block(ui::block(&title))
                    .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
                    .highlight_symbol(">");
                f.render_stateful_widget(l, list, &mut state);
                let help = tr!(
                    " [up/down] move  [enter] select  [key] jump  [mouse] click  [q] quit",
                    " [yukarı/aşağı] gez  [enter] seç  [tuş] atla  [fare] tıkla  [q] çık"
                );
                f.render_widget(
                    Paragraph::new(Span::styled(help, Style::new().fg(ui::dim()))),
                    foot,
                );
            })?;
            if !event::poll(std::time::Duration::from_millis(250))? {
                continue;
            }
            let ev = event::read()?;
            let sel = state.selected().unwrap_or(1);
            let step = |from: usize, down: bool| -> usize {
                let mut i = from;
                loop {
                    i = if down {
                        (i + 1).min(MENU.len() - 1)
                    } else {
                        i.saturating_sub(1)
                    };
                    if selectable(i) || i == 0 || i == MENU.len() - 1 {
                        return if selectable(i) { i } else { from };
                    }
                }
            };
            if let Some(m) = ui::mouse(&ev, list_area, state.offset(), MENU.len()) {
                match m {
                    ui::Mouse::ScrollUp => state.select(Some(step(sel, false))),
                    ui::Mouse::ScrollDown => state.select(Some(step(sel, true))),
                    ui::Mouse::Click { index, .. } if index == sel && selectable(index) => {
                        return Ok((MENU[index].1, index));
                    }
                    ui::Mouse::Click { index, .. } if selectable(index) => {
                        state.select(Some(index))
                    }
                    ui::Mouse::Click { .. } => {}
                }
                continue;
            }
            let Some(code) = ui::pressed(&ev) else {
                continue;
            };
            match code {
                KeyCode::Up | KeyCode::Char('k') => state.select(Some(step(sel, false))),
                KeyCode::Down | KeyCode::Char('j') => state.select(Some(step(sel, true))),
                KeyCode::Enter => return Ok((MENU[sel].1, sel)),
                KeyCode::Esc => return Ok(("quit", sel)),
                KeyCode::Char(c) => {
                    if let Some(i) = MENU.iter().position(|m| m.0 != ' ' && m.0 == c) {
                        return Ok((MENU[i].1, i));
                    }
                }
                _ => {}
            }
        }
    })();
    ui::tui_end();
    res
}

fn pause() {
    if ui::interactive() {
        println!(
            "  {}",
            tr!(
                "press enter to return to the menu...",
                "menüye dönmek için enter'a basın..."
            )
        );
        let _ = std::io::stdin().read_line(&mut String::new());
    }
}

fn run_menu_item(name: &str) -> anyhow::Result<bool> {
    let wait = !matches!(name, "analyze" | "status");
    match name {
        "clean" => clean::run(false, false)?,
        "analyze" => analyze::run(None)?,
        "purge" => purge::run(vec![], false, false)?,
        "installer" => installer::run(vec![], false, false)?,
        "dupes" => dupes::run(vec![], 1, false)?,
        "empty" => empty::run(vec![], false)?,
        "privacy" => privacy::run(false, false)?,
        "uninstall" => uninstall::run(false, false)?,
        "upgrade" => upgrade::run(false, false)?,
        "drivers" => drivers::run(false, false, false, false)?,
        "startup" => startup::run(false)?,
        "debloat" => debloat::run(false, false)?,
        "backup" => backup::create(None, false)?,
        "health" => health::run()?,
        "status" => status::run()?,
        "optimize" => optimize::run(false, false)?,
        "power" => power::run(None, None)?,
        "restore" => restore::run(None, None)?,
        "report" => report::run(None, true)?,
        "bench" => bench::run(None, 1024)?,
        "schedule" => schedule::run(None, "11:00".into())?,
        "net" => net::run(None, None, None, false)?,
        "ports" => ports::run(None, None)?,
        "whitelist" => whitelist::run(vec![], vec![], false)?,
        "rules" => clean::rules(false)?,
        "context" => context::run(None)?,
        "config" => settings::run(None, None)?,
        "history" => oplog::show_history(50),
        "update" => update::run(false)?,
        _ => return Ok(false),
    }
    if wait {
        pause();
    }
    Ok(true)
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    update::cleanup_old();
    ui::init();
    // Load protected paths first; an unreadable whitelist stops everything.
    whitelist::init()?;
    match cli.cmd {
        Some(Cmd::Clean { dry_run, yes }) => clean::run(dry_run, yes),
        Some(Cmd::Analyze { path }) => analyze::run(path),
        Some(Cmd::Status) => status::run(),
        Some(Cmd::Purge {
            paths,
            dry_run,
            yes,
        }) => purge::run(paths, dry_run, yes),
        Some(Cmd::Installer {
            paths,
            dry_run,
            yes,
        }) => installer::run(paths, dry_run, yes),
        Some(Cmd::Uninstall { dry_run, quiet }) => uninstall::run(dry_run, quiet),
        Some(Cmd::Upgrade { dry_run, yes }) => upgrade::run(dry_run, yes),
        Some(Cmd::Drivers {
            installed,
            dry_run,
            yes,
            no_restore_point,
        }) => drivers::run(installed, dry_run, yes, no_restore_point),
        Some(Cmd::Optimize { dry_run, yes }) => optimize::run(dry_run, yes),
        Some(Cmd::Whitelist { add, remove, edit }) => whitelist::run(add, remove, edit),
        Some(Cmd::Startup { list }) => startup::run(list),
        Some(Cmd::Health) => health::run(),
        Some(Cmd::Dupes {
            paths,
            min_mb,
            dry_run,
        }) => dupes::run(paths, min_mb, dry_run),
        Some(Cmd::Debloat { dry_run, restore }) => debloat::run(dry_run, restore),
        Some(Cmd::Privacy { dry_run, yes }) => privacy::run(dry_run, yes),
        Some(Cmd::Schedule { frequency, time }) => schedule::run(frequency, time),
        Some(Cmd::Update { check }) => update::run(check),
        Some(Cmd::Config { key, value }) => settings::run(key, value),
        Some(Cmd::Backup {
            action,
            folder,
            dry_run,
        }) => match action.as_deref() {
            Some("restore") => {
                let f = folder.ok_or_else(|| anyhow::anyhow!("kz backup restore FOLDER"))?;
                backup::restore(&f, dry_run)
            }
            Some(dir) => backup::create(Some(PathBuf::from(dir)), dry_run),
            None => backup::create(None, dry_run),
        },
        Some(Cmd::Ports { filter, kill }) => ports::run(filter, kill),
        Some(Cmd::Net {
            action,
            arg,
            arg2,
            show,
        }) => net::run(action, arg, arg2, show),
        Some(Cmd::Empty { paths, dry_run }) => empty::run(paths, dry_run),
        Some(Cmd::Power { action, arg }) => power::run(action, arg),
        Some(Cmd::Report { output, no_open }) => report::run(output, !no_open),
        Some(Cmd::Restore {
            action,
            description,
        }) => restore::run(action, description),
        Some(Cmd::Context { action }) => context::run(action),
        Some(Cmd::Bench { path, size_mb }) => bench::run(path, size_mb),
        Some(Cmd::Rules { edit }) => clean::rules(edit),
        Some(Cmd::Remove { dry_run, yes }) => remove::run(dry_run, yes),
        Some(Cmd::History { limit }) => {
            oplog::show_history(limit);
            Ok(())
        }
        None => {
            let notice = update::spawn_check();
            let mut newer: Option<String> = None;
            let mut pos = 1;
            loop {
                let (item, at) = menu(pos, &notice, &mut newer)?;
                pos = at;
                match run_menu_item(item) {
                    Ok(true) => {}
                    Ok(false) => return Ok(()),
                    Err(e) => {
                        ui::warn(&format!("{e}"));
                        pause();
                    }
                }
            }
        }
    }
}
