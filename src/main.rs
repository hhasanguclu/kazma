//! kazma: an oldschool, all-ASCII system cleaner and disk explorer for Windows.

mod analyze;
mod clean;
mod drivers;
mod fsutil;
mod installer;
mod oplog;
mod optimize;
mod ps;
mod purge;
mod remove;
mod status;
mod sys;
mod ui;
mod uninstall;
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
#[command(name = "kz", version, about = "Oldschool ASCII system cleaner & disk explorer for Windows")]
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

/// (hotkey, command, description)
const MENU: [(char, &str, &str); 12] = [
    ('1', "clean", "free up space: caches, temp files, logs, recycle bin"),
    ('2', "analyze", "explore what is using your disk"),
    ('3', "status", "live cpu / memory / disk / network monitor"),
    ('4', "purge", "delete project build artifacts (node_modules, target...)"),
    ('5', "installer", "remove old setup files from Downloads / Desktop"),
    ('6', "uninstall", "remove programs and their leftovers"),
    ('7', "upgrade", "update installed programs (winget)"),
    ('8', "drivers", "driver updates from windows update"),
    ('9', "optimize", "dns flush, icon cache, ssd trim, dism, sfc"),
    ('w', "whitelist", "paths kazma must never delete"),
    ('h', "history", "show past operations"),
    ('q', "quit", "exit kazma"),
];

fn menu() -> anyhow::Result<&'static str> {
    let mut terminal = ratatui::init();
    let mut state = ListState::default().with_selected(Some(0));
    let res = (|| -> anyhow::Result<&'static str> {
        loop {
            terminal.draw(|f| {
                let banner_h = ui::BANNER.lines().count() as u16 + 2;
                let [top, list, foot] = Layout::vertical([
                    Constraint::Length(banner_h),
                    Constraint::Length(MENU.len() as u16 + 2),
                    Constraint::Min(1),
                ])
                .areas(f.area());
                let mut lines: Vec<Line> =
                    ui::BANNER.lines().map(|l| Line::from(Span::styled(format!("  {l}"), Style::new().fg(ui::FG)))).collect();
                lines.push(Line::from(Span::styled(
                    format!("  v{}  ::  oldschool system toolkit for windows", env!("CARGO_PKG_VERSION")),
                    Style::new().fg(ui::DIM),
                )));
                f.render_widget(Paragraph::new(lines), top);

                let items: Vec<ListItem> = MENU
                    .iter()
                    .map(|(key, name, desc)| {
                        ListItem::new(Line::from(vec![
                            Span::styled(format!(" {key}. "), Style::new().fg(ui::DIM)),
                            Span::styled(format!("{name:<11}"), Style::new().fg(ui::HI).add_modifier(Modifier::BOLD)),
                            Span::styled(*desc, Style::new().fg(ui::FG)),
                        ]))
                    })
                    .collect();
                let l = List::new(items)
                    .block(ui::block("main menu"))
                    .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
                    .highlight_symbol(">");
                f.render_stateful_widget(l, list, &mut state);
                f.render_widget(
                    Paragraph::new(Span::styled(" [up/down] move  [enter] select  [key] jump  [q] quit", Style::new().fg(ui::DIM))),
                    foot,
                );
            })?;
            let Some(code) = ui::pressed(&event::read()?) else { continue };
            let sel = state.selected().unwrap_or(0);
            match code {
                KeyCode::Up | KeyCode::Char('k') => state.select(Some(sel.saturating_sub(1))),
                KeyCode::Down | KeyCode::Char('j') => state.select(Some((sel + 1).min(MENU.len() - 1))),
                KeyCode::Enter => return Ok(MENU[sel].1),
                KeyCode::Esc => return Ok("quit"),
                KeyCode::Char(c) => {
                    if let Some(m) = MENU.iter().find(|m| m.0 == c) {
                        return Ok(m.1);
                    }
                }
                _ => {}
            }
        }
    })();
    ratatui::restore();
    res
}

fn pause() {
    if ui::interactive() {
        println!("  press enter to return to the menu...");
        let _ = std::io::stdin().read_line(&mut String::new());
    }
}

fn run_menu_item(name: &str) -> anyhow::Result<bool> {
    let wait = !matches!(name, "analyze" | "status");
    match name {
        "clean" => clean::run(false, false)?,
        "analyze" => analyze::run(None)?,
        "status" => status::run()?,
        "purge" => purge::run(vec![], false, false)?,
        "installer" => installer::run(vec![], false, false)?,
        "uninstall" => uninstall::run(false, false)?,
        "upgrade" => upgrade::run(false, false)?,
        "drivers" => drivers::run(false, false, false, false)?,
        "optimize" => optimize::run(false, false)?,
        "whitelist" => whitelist::run(vec![], vec![], false)?,
        "history" => oplog::show_history(50),
        _ => return Ok(false),
    }
    if wait {
        pause();
    }
    Ok(true)
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    // Load protected paths first; an unreadable whitelist stops everything.
    whitelist::init()?;
    match cli.cmd {
        Some(Cmd::Clean { dry_run, yes }) => clean::run(dry_run, yes),
        Some(Cmd::Analyze { path }) => analyze::run(path),
        Some(Cmd::Status) => status::run(),
        Some(Cmd::Purge { paths, dry_run, yes }) => purge::run(paths, dry_run, yes),
        Some(Cmd::Installer { paths, dry_run, yes }) => installer::run(paths, dry_run, yes),
        Some(Cmd::Uninstall { dry_run, quiet }) => uninstall::run(dry_run, quiet),
        Some(Cmd::Upgrade { dry_run, yes }) => upgrade::run(dry_run, yes),
        Some(Cmd::Drivers { installed, dry_run, yes, no_restore_point }) => drivers::run(installed, dry_run, yes, no_restore_point),
        Some(Cmd::Optimize { dry_run, yes }) => optimize::run(dry_run, yes),
        Some(Cmd::Whitelist { add, remove, edit }) => whitelist::run(add, remove, edit),
        Some(Cmd::Remove { dry_run, yes }) => remove::run(dry_run, yes),
        Some(Cmd::History { limit }) => {
            oplog::show_history(limit);
            Ok(())
        }
        None => loop {
            let item = menu()?;
            match run_menu_item(item) {
                Ok(true) => {}
                Ok(false) => return Ok(()),
                Err(e) => {
                    ui::warn(&format!("{e}"));
                    pause();
                }
            }
        },
    }
}
