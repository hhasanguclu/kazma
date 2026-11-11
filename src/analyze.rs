//! `kz analyze`: interactive disk usage explorer.
//! Children are listed instantly; their sizes stream in from a background thread pool.

use crate::{fsutil, oplog, sys, tr, ui};
use crossterm::event;
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, SystemTime};

struct Entry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: Option<u64>,
    modified: Option<SystemTime>,
}

#[derive(Clone, Copy, PartialEq)]
enum SortKey {
    Size,
    Name,
    Date,
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
    /// "largest files" mode: a flat list of the biggest files under `cwd`.
    big: bool,
    big_rx: Option<Receiver<Vec<(PathBuf, u64)>>>,
    /// Indices into `entries` that pass the search filter, in display order.
    view: Vec<usize>,
    search: ui::Search,
    sort_key: SortKey,
    list_area: Rect,
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
            big: false,
            big_rx: None,
            view: vec![],
            search: ui::Search::default(),
            sort_key: SortKey::Size,
            list_area: Rect::default(),
        };
        app.load(start, 0);
        app
    }

    fn load(&mut self, dir: PathBuf, select: usize) {
        self.big = false;
        self.big_rx = None;
        self.search = ui::Search::default();
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
                        let meta = e.metadata().ok();
                        let size = if ft.is_dir() {
                            self.cache.get(&path).copied()
                        } else {
                            meta.as_ref().map(|m| m.len())
                        };
                        Some(Entry {
                            name: e.file_name().to_string_lossy().into_owned(),
                            path,
                            is_dir: ft.is_dir(),
                            size,
                            modified: meta.and_then(|m| m.modified().ok()),
                        })
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
        self.state.select(
            (!self.view.is_empty()).then_some(select.min(self.view.len().saturating_sub(1))),
        );
    }

    fn refresh_view(&mut self) {
        self.view = (0..self.entries.len())
            .filter(|&i| self.search.matches(&self.entries[i].name))
            .collect();
        if self.state.selected().is_none_or(|s| s >= self.view.len()) {
            self.state.select((!self.view.is_empty()).then_some(0));
        }
    }

    fn sort(&mut self) {
        let sel = self.selected().map(|e| e.path.clone());
        match self.sort_key {
            SortKey::Size => self.entries.sort_by(|a, b| {
                b.size
                    .unwrap_or(0)
                    .cmp(&a.size.unwrap_or(0))
                    .then(a.name.cmp(&b.name))
            }),
            SortKey::Name => self.entries.sort_by(|a, b| {
                (!a.is_dir, a.name.to_lowercase()).cmp(&(!b.is_dir, b.name.to_lowercase()))
            }),
            SortKey::Date => self.entries.sort_by_key(|e| std::cmp::Reverse(e.modified)),
        }
        self.refresh_view();
        if self.pinned_top && !self.view.is_empty() {
            self.state.select(Some(0));
        } else if let Some(p) = sel {
            let pos = self.view.iter().position(|&i| self.entries[i].path == p);
            self.state
                .select(pos.or((!self.view.is_empty()).then_some(0)));
        }
    }

    fn start_big(&mut self) {
        let (tx, rx) = channel();
        let dir = self.cwd.clone();
        rayon::spawn(move || {
            let _ = tx.send(largest_files(&dir, BIG_COUNT));
        });
        self.big = true;
        self.big_rx = Some(rx);
        self.entries.clear();
        self.search = ui::Search::default();
        self.view.clear();
        self.state.select(None);
    }

    fn drain(&mut self) {
        if let Some(rx) = &self.big_rx
            && let Ok(list) = rx.try_recv()
        {
            let base = self.cwd.clone();
            self.entries = list
                .into_iter()
                .map(|(path, size)| Entry {
                    name: path
                        .strip_prefix(&base)
                        .unwrap_or(&path)
                        .display()
                        .to_string(),
                    path,
                    is_dir: false,
                    size: Some(size),
                    modified: None,
                })
                .collect();
            self.big_rx = None;
            self.refresh_view();
            self.state.select((!self.view.is_empty()).then_some(0));
        }
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
        self.state
            .selected()
            .and_then(|i| self.view.get(i))
            .and_then(|&i| self.entries.get(i))
    }

    fn enter(&mut self) {
        if self.big {
            if let Some(e) = self.selected() {
                sys::reveal(&e.path);
            }
            return;
        }
        if let Some(e) = self.selected().filter(|e| e.is_dir) {
            let p = e.path.clone();
            self.history
                .push((self.cwd.clone(), self.state.selected().unwrap_or(0)));
            self.load(p, 0);
        }
    }

    fn back(&mut self) {
        if self.big {
            let cwd = self.cwd.clone();
            self.load(cwd, 0);
            return;
        }
        if let Some((p, sel)) = self.history.pop() {
            self.load(p, sel);
        } else if let Some(parent) = self.cwd.parent().map(Path::to_path_buf) {
            let child = self.cwd.clone();
            self.load(parent, 0);
            let pos = self
                .view
                .iter()
                .position(|&i| self.entries[i].path == child);
            self.state.select(pos);
        }
    }

    fn delete_selected(&mut self) {
        let Some(e) = self.selected() else { return };
        let (path, size) = (e.path.clone(), e.size.unwrap_or(0));
        if !fsutil::can_remove(&path) {
            self.message = tr!(
                "refused: {} is protected (system folder or whitelist)",
                "reddedildi: {} korumalı (sistem klasörü ya da whitelist)",
                path.display()
            );
            return;
        }
        match fsutil::trash(&path) {
            Ok(()) => {
                self.message = tr!(
                    "moved to Recycle Bin: {} ({})",
                    "Geri Dönüşüm Kutusu'na taşındı: {} ({})",
                    path.display(),
                    ui::human(size)
                );
                oplog::record(
                    "analyze",
                    &format!("trashed {} size={size}", path.display()),
                );
                self.invalidate_ancestors(&path);
                let sel = self.state.selected().unwrap_or(0);
                if self.big {
                    if let Some(&i) = self.view.get(sel) {
                        self.entries.remove(i);
                    }
                    self.refresh_view();
                    self.state.select(
                        (!self.view.is_empty())
                            .then_some(sel.min(self.view.len().saturating_sub(1))),
                    );
                } else {
                    let cwd = self.cwd.clone();
                    self.load(cwd, sel);
                }
            }
            Err(err) => self.message = tr!("delete failed: {err}", "silinemedi: {err}"),
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
        .or_else(|| {
            fsutil::env_path("SystemDrive").map(|d| PathBuf::from(format!("{}\\", d.display())))
        })
        .or_else(fsutil::home)
        .unwrap_or_else(|| PathBuf::from("."));
    let start = std::fs::canonicalize(&start).unwrap_or(start);
    let mut app = App::new(start);
    let mut terminal = ui::tui_start();
    let res = (|| -> anyhow::Result<()> {
        loop {
            app.drain();
            terminal.draw(|f| draw(f, &mut app))?;
            if !event::poll(Duration::from_millis(120))? {
                continue;
            }
            let ev = event::read()?;
            if !app.confirm_delete
                && let Some(m) = ui::mouse(&ev, app.list_area, app.state.offset(), app.view.len())
            {
                app.pinned_top = false;
                let sel = app.state.selected().unwrap_or(0);
                let last = app.view.len().saturating_sub(1);
                match m {
                    ui::Mouse::ScrollUp => app.state.select(Some(sel.saturating_sub(3))),
                    ui::Mouse::ScrollDown => app.state.select(Some((sel + 3).min(last))),
                    ui::Mouse::Click { index, .. } if index == sel => app.enter(),
                    ui::Mouse::Click { index, .. } => app.state.select(Some(index)),
                }
                continue;
            }
            let Some(code) = ui::pressed(&ev) else {
                continue;
            };
            if app.confirm_delete {
                app.confirm_delete = false;
                if matches!(code, KeyCode::Char('y') | KeyCode::Char('Y')) {
                    app.delete_selected();
                } else {
                    app.message = ui::t("delete cancelled", "silme iptal edildi").into();
                }
                continue;
            }
            app.message.clear();
            if app.search.key(code) {
                app.pinned_top = false;
                app.refresh_view();
                app.state.select((!app.view.is_empty()).then_some(0));
                continue;
            }
            if matches!(
                code,
                KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Char('j' | 'k' | 'g' | 'G')
                    | KeyCode::PageUp
                    | KeyCode::PageDown
                    | KeyCode::Home
                    | KeyCode::End
            ) {
                app.pinned_top = false;
            }
            let n = app.view.len();
            let sel = app.state.selected().unwrap_or(0);
            match code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Up | KeyCode::Char('k') => app.state.select(Some(sel.saturating_sub(1))),
                KeyCode::Down | KeyCode::Char('j') => {
                    app.state.select(Some((sel + 1).min(n.saturating_sub(1))))
                }
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
                KeyCode::Char('b') => {
                    if app.big {
                        app.back();
                    } else {
                        app.start_big();
                    }
                }
                KeyCode::Char('s') if !app.big => {
                    app.sort_key = match app.sort_key {
                        SortKey::Size => SortKey::Name,
                        SortKey::Name => SortKey::Date,
                        SortKey::Date => SortKey::Size,
                    };
                    app.pinned_top = true;
                    app.sort();
                }
                KeyCode::Char('r') if app.big => app.start_big(),
                KeyCode::Char('r') => {
                    let cwd = app.cwd.clone();
                    app.invalidate_ancestors(&cwd);
                    let keys: Vec<PathBuf> = app
                        .cache
                        .keys()
                        .filter(|k| k.starts_with(&cwd))
                        .cloned()
                        .collect();
                    keys.iter().for_each(|k| {
                        app.cache.remove(k);
                    });
                    app.load(cwd, sel);
                }
                _ => {}
            }
        }
    })();
    ui::tui_end();
    res
}

fn draw(f: &mut ratatui::Frame, app: &mut App) {
    let [head, main, foot] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(3),
    ])
    .areas(f.area());

    let pending = app.entries.iter().filter(|e| e.size.is_none()).count();
    let total: u64 = app.entries.iter().filter_map(|e| e.size).sum();
    let status = if app.big_rx.is_some() {
        tr!(
            "  finding the largest files...",
            "  en büyük dosyalar aranıyor..."
        )
    } else if app.big {
        tr!(
            "  largest {} files",
            "  en büyük {} dosya",
            app.entries.len()
        )
    } else if pending > 0 {
        tr!("  scanning {pending}...", "  taranıyor {pending}...")
    } else {
        String::new()
    };
    let sort = match app.sort_key {
        SortKey::Size => ui::t("size", "boyut"),
        SortKey::Name => ui::t("name", "ad"),
        SortKey::Date => ui::t("date", "tarih"),
    };
    let status = if app.big {
        status
    } else {
        format!("{status}  [{}: {sort}]", ui::t("sort", "sıra"))
    };
    let header = Line::from(vec![
        Span::styled(
            format!(" {} ", app.cwd.display()),
            Style::new().fg(ui::fg()).bold(),
        ),
        Span::styled(
            tr!(" total {}", " toplam {}", ui::human(total)),
            Style::new().fg(ui::hi()),
        ),
        Span::styled(status, Style::new().fg(ui::dim())),
    ]);
    f.render_widget(
        Paragraph::new(header).block(ui::block("kazma analyze")),
        head,
    );

    let max = app
        .entries
        .iter()
        .filter_map(|e| e.size)
        .max()
        .unwrap_or(1)
        .max(1);
    let width = main.width.saturating_sub(4) as usize;
    let bar_w = 20usize;
    let name_w = width.saturating_sub(bar_w + 2 + 10 + 2 + 7 + 3);
    app.list_area = main;
    let items: Vec<ListItem> = app
        .view
        .iter()
        .map(|&i| &app.entries[i])
        .map(|e| {
            let (size, bar, pct) = match e.size {
                Some(s) => (
                    ui::human(s),
                    ui::bar(s as f64 / max as f64, bar_w),
                    if total > 0 {
                        format!("{:>5.1}%", s as f64 * 100.0 / total as f64)
                    } else {
                        "     -".into()
                    },
                ),
                None => (
                    "...".into(),
                    format!("[{}]", "?".repeat(bar_w)),
                    "     -".into(),
                ),
            };
            let name = if e.is_dir {
                format!("{}{}", e.name, std::path::MAIN_SEPARATOR)
            } else {
                e.name.clone()
            };
            ListItem::new(Line::from(vec![
                Span::styled(bar, Style::new().fg(ui::fg())),
                Span::styled(format!(" {size:>10} "), Style::new().fg(ui::hi())),
                Span::styled(format!("{pct} "), Style::new().fg(ui::dim())),
                Span::styled(
                    ui::truncate(&name, name_w),
                    if e.is_dir {
                        Style::new().fg(ui::fg()).add_modifier(Modifier::BOLD)
                    } else {
                        Style::new().fg(ui::fg())
                    },
                ),
            ]))
        })
        .collect();
    let title = if app.view.len() == app.entries.len() {
        tr!("{} items", "{} öğe", app.entries.len())
    } else {
        tr!(
            "{} of {} items",
            "{} / {} öğe",
            app.view.len(),
            app.entries.len()
        )
    };
    let list = List::new(items)
        .block(ui::block(&title))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">");
    f.render_stateful_widget(list, main, &mut app.state);

    let footer = if app.confirm_delete {
        let name = app.selected().map(|e| e.name.clone()).unwrap_or_default();
        Line::from(Span::styled(
            tr!(
                " move '{name}' to Recycle Bin? [y/N]",
                " '{name}' Geri Dönüşüm Kutusu'na taşınsın mı? [y/N]"
            ),
            Style::new().fg(ui::warn_c()).bold(),
        ))
    } else if let Some(s) = app.search.status() {
        Line::from(Span::styled(s, Style::new().fg(ui::hi()).bold()))
    } else if !app.message.is_empty() {
        Line::from(Span::styled(
            format!(" {}", app.message),
            Style::new().fg(ui::hi()),
        ))
    } else {
        let help = if app.big {
            tr!(
                " [enter] show in explorer  [b/bksp] back  [/] search  [d] delete  [r] rescan  [q] quit",
                " [enter] gezginde göster  [b/bksp] geri  [/] ara  [d] sil  [r] yeniden tara  [q] çık"
            )
        } else {
            tr!(
                " [enter] open  [bksp] up  [b] largest  [s] sort  [/] search  [d] delete  [o] explorer  [q] quit",
                " [enter] aç  [bksp] üst  [b] en büyükler  [s] sırala  [/] ara  [d] sil  [o] gezgin  [q] çık"
            )
        };
        Line::from(Span::styled(help, Style::new().fg(ui::dim())))
    };
    let keys = tr!("keys", "tuşlar");
    f.render_widget(Paragraph::new(footer).block(ui::block(&keys)), foot);
}

const BIG_COUNT: usize = 200;

/// The `n` biggest files under `dir` (parallel walk, symlinks not followed).
pub fn largest_files(dir: &Path, n: usize) -> Vec<(PathBuf, u64)> {
    use rayon::prelude::*;
    fn walk(dir: &Path) -> Vec<(PathBuf, u64)> {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return vec![];
        };
        let entries: Vec<std::fs::DirEntry> = rd.flatten().collect();
        entries
            .par_iter()
            .flat_map(|e| {
                let Ok(ft) = e.file_type() else { return vec![] };
                if ft.is_symlink() {
                    vec![]
                } else if ft.is_dir() {
                    walk(&e.path())
                } else {
                    match e.metadata() {
                        Ok(m) if m.len() >= 1024 * 1024 => vec![(e.path(), m.len())],
                        _ => vec![],
                    }
                }
            })
            .collect()
    }
    let mut v = walk(dir);
    v.sort_by_key(|(_, s)| std::cmp::Reverse(*s));
    v.truncate(n);
    v
}
