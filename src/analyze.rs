//! `kz analyze`: interactive disk usage explorer.
//! Children are listed instantly; their sizes stream in from a background thread pool.

use crate::{fsutil, oplog, sys, ui};
use crossterm::event;
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

struct Entry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: Option<u64>,
}

struct App {
    cwd: PathBuf,
    entries: Vec<Entry>,
    state: ListState,
    cache: HashMap<PathBuf, u64>,
    history: Vec<(PathBuf, usize)>,
    tx: Sender<(PathBuf, u64)>,
    rx: Receiver<(PathBuf, u64)>,
    confirm_delete: bool,
    message: String,
    /// Cursor follows the biggest item until the user moves it.
    pinned_top: bool,
}

impl App {
    fn new(start: PathBuf) -> Self {
        let (tx, rx) = channel();
        let mut app = App {
            cwd: start.clone(),
            entries: vec![],
            state: ListState::default(),
            cache: HashMap::new(),
            history: vec![],
            tx,
            rx,
            confirm_delete: false,
            message: String::new(),
            pinned_top: true,
        };
        app.load(start, 0);
        app
    }

    fn load(&mut self, dir: PathBuf, select: usize) {
        self.cwd = dir;
        self.pinned_top = select == 0;
        self.entries = std::fs::read_dir(&self.cwd)
            .map(|rd| {
                rd.flatten()
                    .filter_map(|e| {
                        let ft = e.file_type().ok()?;
                        if ft.is_symlink() {
                            return None;
                        }
                        let path = e.path();
                        let size = if ft.is_dir() { self.cache.get(&path).copied() } else { e.metadata().ok().map(|m| m.len()) };
                        Some(Entry { name: e.file_name().to_string_lossy().into_owned(), path, is_dir: ft.is_dir(), size })
                    })
                    .collect()
            })
            .unwrap_or_default();
        for e in self.entries.iter().filter(|e| e.size.is_none()) {
            let (tx, p) = (self.tx.clone(), e.path.clone());
            rayon::spawn(move || {
                let s = fsutil::dir_size(&p);
                let _ = tx.send((p, s));
            });
        }
        self.sort();
        self.state.select((!self.entries.is_empty()).then_some(select.min(self.entries.len().saturating_sub(1))));
    }

    fn sort(&mut self) {
        let sel = self.state.selected().and_then(|i| self.entries.get(i)).map(|e| e.path.clone());
        self.entries.sort_by(|a, b| b.size.unwrap_or(0).cmp(&a.size.unwrap_or(0)).then(a.name.cmp(&b.name)));
        if self.pinned_top && !self.entries.is_empty() {
            self.state.select(Some(0));
        } else if let Some(p) = sel {
            self.state.select(self.entries.iter().position(|e| e.path == p));
        }
    }

    fn drain(&mut self) {
        let mut changed = false;
        while let Ok((p, s)) = self.rx.try_recv() {
            self.cache.insert(p.clone(), s);
            if let Some(e) = self.entries.iter_mut().find(|e| e.path == p) {
                e.size = Some(s);
                changed = true;
            }
        }
        if changed {
            self.sort();
        }
    }

    fn selected(&self) -> Option<&Entry> {
        self.state.selected().and_then(|i| self.entries.get(i))
    }

    fn enter(&mut self) {
        if let Some(e) = self.selected().filter(|e| e.is_dir) {
            let p = e.path.clone();
            self.history.push((self.cwd.clone(), self.state.selected().unwrap_or(0)));
            self.load(p, 0);
        }
    }

    fn back(&mut self) {
        if let Some((p, sel)) = self.history.pop() {
            self.load(p, sel);
        } else if let Some(parent) = self.cwd.parent().map(Path::to_path_buf) {
            let child = self.cwd.clone();
            self.load(parent, 0);
            self.state.select(self.entries.iter().position(|e| e.path == child));
        }
    }

    fn delete_selected(&mut self) {
        let Some(e) = self.selected() else { return };
        let (path, size) = (e.path.clone(), e.size.unwrap_or(0));
        if !fsutil::can_remove(&path) {
            self.message = format!("refused: {} is protected (system folder or whitelist)", path.display());
            return;
        }
        match fsutil::trash(&path) {
            Ok(()) => {
                self.message = format!("moved to Recycle Bin: {} ({})", path.display(), ui::human(size));
                oplog::record("analyze", &format!("trashed {} size={size}", path.display()));
                self.invalidate_ancestors(&path);
                let sel = self.state.selected().unwrap_or(0);
                let cwd = self.cwd.clone();
                self.load(cwd, sel);
            }
            Err(err) => self.message = format!("delete failed: {err}"),
        }
    }

    fn invalidate_ancestors(&mut self, path: &Path) {
        self.cache.remove(path);
        let mut p = path.parent();
        while let Some(a) = p {
            self.cache.remove(a);
            p = a.parent();
        }
    }
}

pub fn run(path: Option<PathBuf>) -> anyhow::Result<()> {
    let start = path
        .or_else(|| fsutil::env_path("SystemDrive").map(|d| PathBuf::from(format!("{}\\", d.display()))))
        .or_else(fsutil::home)
        .unwrap_or_else(|| PathBuf::from("."));
    let start = std::fs::canonicalize(&start).unwrap_or(start);
    let mut app = App::new(start);
    let mut terminal = ratatui::init();
    let res = (|| -> anyhow::Result<()> {
        loop {
            app.drain();
            terminal.draw(|f| draw(f, &mut app))?;
            if !event::poll(Duration::from_millis(120))? {
                continue;
            }
            let ev = event::read()?;
            let Some(code) = ui::pressed(&ev) else { continue };
            if app.confirm_delete {
                app.confirm_delete = false;
                if matches!(code, KeyCode::Char('y') | KeyCode::Char('Y')) {
                    app.delete_selected();
                } else {
                    app.message = "delete cancelled".into();
                }
                continue;
            }
            app.message.clear();
            if matches!(code, KeyCode::Up | KeyCode::Down | KeyCode::Char('j' | 'k' | 'g' | 'G') | KeyCode::PageUp | KeyCode::PageDown | KeyCode::Home | KeyCode::End) {
                app.pinned_top = false;
            }
            let n = app.entries.len();
            let sel = app.state.selected().unwrap_or(0);
            match code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Up | KeyCode::Char('k') => app.state.select(Some(sel.saturating_sub(1))),
                KeyCode::Down | KeyCode::Char('j') => app.state.select(Some((sel + 1).min(n.saturating_sub(1)))),
                KeyCode::PageUp => app.state.select(Some(sel.saturating_sub(15))),
                KeyCode::PageDown => app.state.select(Some((sel + 15).min(n.saturating_sub(1)))),
                KeyCode::Home | KeyCode::Char('g') => app.state.select(Some(0)),
                KeyCode::End | KeyCode::Char('G') => app.state.select(Some(n.saturating_sub(1))),
                KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => app.enter(),
                KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => app.back(),
                KeyCode::Char('o') => {
                    if let Some(e) = app.selected() {
                        sys::reveal(&e.path);
                    }
                }
                KeyCode::Char('d') | KeyCode::Delete => {
                    if app.selected().is_some() {
                        app.confirm_delete = true;
                    }
                }
                KeyCode::Char('r') => {
                    let cwd = app.cwd.clone();
                    app.invalidate_ancestors(&cwd);
                    let keys: Vec<PathBuf> = app.cache.keys().filter(|k| k.starts_with(&cwd)).cloned().collect();
                    keys.iter().for_each(|k| { app.cache.remove(k); });
                    app.load(cwd, sel);
                }
                _ => {}
            }
        }
    })();
    ratatui::restore();
    res
}

fn draw(f: &mut ratatui::Frame, app: &mut App) {
    let [head, main, foot] =
        Layout::vertical([Constraint::Length(3), Constraint::Min(3), Constraint::Length(3)]).areas(f.area());

    let pending = app.entries.iter().filter(|e| e.size.is_none()).count();
    let total: u64 = app.entries.iter().filter_map(|e| e.size).sum();
    let status = if pending > 0 { format!("  scanning {pending}...") } else { String::new() };
    let header = Line::from(vec![
        Span::styled(format!(" {} ", app.cwd.display()), Style::new().fg(ui::FG).bold()),
        Span::styled(format!(" total {}", ui::human(total)), Style::new().fg(ui::HI)),
        Span::styled(status, Style::new().fg(ui::DIM)),
    ]);
    f.render_widget(Paragraph::new(header).block(ui::block("kazma analyze")), head);

    let max = app.entries.iter().filter_map(|e| e.size).max().unwrap_or(1).max(1);
    let width = main.width.saturating_sub(4) as usize;
    let bar_w = 20usize;
    let name_w = width.saturating_sub(bar_w + 2 + 10 + 2 + 7 + 3);
    let items: Vec<ListItem> = app
        .entries
        .iter()
        .map(|e| {
            let (size, bar, pct) = match e.size {
                Some(s) => (
                    ui::human(s),
                    ui::bar(s as f64 / max as f64, bar_w),
                    if total > 0 { format!("{:>5.1}%", s as f64 * 100.0 / total as f64) } else { "     -".into() },
                ),
                None => ("...".into(), format!("[{}]", "?".repeat(bar_w)), "     -".into()),
            };
            let name = if e.is_dir { format!("{}{}", e.name, std::path::MAIN_SEPARATOR) } else { e.name.clone() };
            ListItem::new(Line::from(vec![
                Span::styled(bar, Style::new().fg(ui::FG)),
                Span::styled(format!(" {size:>10} "), Style::new().fg(ui::HI)),
                Span::styled(format!("{pct} "), Style::new().fg(ui::DIM)),
                Span::styled(
                    ui::truncate(&name, name_w),
                    if e.is_dir { Style::new().fg(ui::FG).add_modifier(Modifier::BOLD) } else { Style::new().fg(ui::FG) },
                ),
            ]))
        })
        .collect();
    let title = format!("{} items", app.entries.len());
    let list = List::new(items)
        .block(ui::block(&title))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">");
    f.render_stateful_widget(list, main, &mut app.state);

    let footer = if app.confirm_delete {
        let name = app.selected().map(|e| e.name.clone()).unwrap_or_default();
        Line::from(Span::styled(format!(" move '{name}' to Recycle Bin? [y/N]"), Style::new().fg(ui::WARN).bold()))
    } else if !app.message.is_empty() {
        Line::from(Span::styled(format!(" {}", app.message), Style::new().fg(ui::HI)))
    } else {
        Line::from(Span::styled(
            " [enter] open  [bksp] up  [d] delete  [o] explorer  [r] rescan  [q] quit",
            Style::new().fg(ui::DIM),
        ))
    };
    f.render_widget(Paragraph::new(footer).block(ui::block("keys")), foot);
}
