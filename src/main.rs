//! kazma: an oldschool, all-ASCII system cleaner and disk explorer for Windows.

mod analyze;
mod clean;
mod fsutil;
mod oplog;
mod purge;
mod status;
mod sys;
mod ui;

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
    /// Show the log of past cleanup operations
    History {
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
}

const MENU: [(&str, &str); 6] = [
    ("clean", "free up space: caches, temp files, logs, recycle bin"),
    ("analyze", "explore what is using your disk"),
    ("status", "live cpu / memory / disk / network monitor"),
    ("purge", "delete project build artifacts (node_modules, target...)"),
    ("history", "show past operations"),
    ("quit", "exit kazma"),
];

fn menu() -> anyhow::Result<Option<usize>> {
    let mut terminal = ratatui::init();
    let mut state = ListState::default().with_selected(Some(0));
    let res = (|| -> anyhow::Result<Option<usize>> {
        loop {
            terminal.draw(|f| {
                let banner_h = ui::BANNER.lines().count() as u16 + 2;
                let [top, list, foot] = Layout::vertical([
                    Constraint::Length(banner_h + 2),
                    Constraint::Length(MENU.len() as u16 + 2),
                    Constraint::Min(1),
                ])
                .areas(f.area());
                let mut lines: Vec<Line> =
                    ui::BANNER.lines().map(|l| Line::from(Span::styled(format!("  {l}"), Style::new().fg(ui::FG)))).collect();
                lines.push(Line::from(Span::styled(
                    format!("  v{}  ::  disk cleaner & explorer for windows", env!("CARGO_PKG_VERSION")),
                    Style::new().fg(ui::DIM),
                )));
                f.render_widget(Paragraph::new(lines), top);

                let items: Vec<ListItem> = MENU
                    .iter()
                    .enumerate()
                    .map(|(i, (name, desc))| {
                        ListItem::new(Line::from(vec![
                            Span::styled(format!(" {}. ", i + 1), Style::new().fg(ui::DIM)),
                            Span::styled(format!("{name:<9}"), Style::new().fg(ui::HI).add_modifier(Modifier::BOLD)),
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
                    Paragraph::new(Span::styled(" [up/down] move  [enter] select  [1-6] jump  [q] quit", Style::new().fg(ui::DIM))),
                    foot,
                );
            })?;
            let Some(code) = ui::pressed(&event::read()?) else { continue };
            let sel = state.selected().unwrap_or(0);
            match code {
                KeyCode::Up | KeyCode::Char('k') => state.select(Some(sel.saturating_sub(1))),
                KeyCode::Down | KeyCode::Char('j') => state.select(Some((sel + 1).min(MENU.len() - 1))),
                KeyCode::Enter => return Ok(Some(sel)),
                KeyCode::Char(c @ '1'..='6') => return Ok(Some(c as usize - '1' as usize)),
                KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Some(Cmd::Clean { dry_run, yes }) => clean::run(dry_run, yes),
        Some(Cmd::Analyze { path }) => analyze::run(path),
        Some(Cmd::Status) => status::run(),
        Some(Cmd::Purge { paths, dry_run, yes }) => purge::run(paths, dry_run, yes),
        Some(Cmd::History { limit }) => {
            oplog::show_history(limit);
            Ok(())
        }
        None => loop {
            match menu()? {
                Some(0) => { clean::run(false, false)?; pause(); }
                Some(1) => analyze::run(None)?,
                Some(2) => status::run()?,
                Some(3) => { purge::run(vec![], false, false)?; pause(); }
                Some(4) => { oplog::show_history(50); pause(); }
                _ => return Ok(()),
            }
        },
    }
}
