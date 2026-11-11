//! Retro ASCII look: banner, borders, colors, size formatting and a checklist widget.

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::style::Stylize;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};
use std::io::IsTerminal;
use std::sync::OnceLock;

// ---- theme -------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct Theme {
    pub fg: Color,
    pub hi: Color,
    pub dim: Color,
    pub warn: Color,
}

pub fn theme_by_name(name: &str) -> Theme {
    match name {
        "amber" => Theme {
            fg: Color::Indexed(214),
            hi: Color::Indexed(229),
            dim: Color::Indexed(94),
            warn: Color::Indexed(196),
        },
        "white" => Theme {
            fg: Color::Gray,
            hi: Color::White,
            dim: Color::DarkGray,
            warn: Color::Red,
        },
        "blue" => Theme {
            fg: Color::Cyan,
            hi: Color::Indexed(153),
            dim: Color::Indexed(24),
            warn: Color::Red,
        },
        _ => Theme {
            fg: Color::Green,
            hi: Color::Yellow,
            dim: Color::DarkGray,
            warn: Color::Red,
        },
    }
}

static THEME: OnceLock<Theme> = OnceLock::new();
static TURKISH: OnceLock<bool> = OnceLock::new();

fn theme() -> Theme {
    *THEME.get_or_init(|| theme_by_name("green"))
}
pub fn fg() -> Color {
    theme().fg
}
pub fn hi() -> Color {
    theme().hi
}
pub fn dim() -> Color {
    theme().dim
}
pub fn warn_c() -> Color {
    theme().warn
}

fn ct(c: Color) -> crossterm::style::Color {
    use crossterm::style::Color as C;
    match c {
        Color::Green => C::Green,
        Color::Yellow => C::Yellow,
        Color::DarkGray => C::DarkGrey,
        Color::Red => C::Red,
        Color::Gray => C::Grey,
        Color::White => C::White,
        Color::Cyan => C::Cyan,
        Color::Indexed(n) => C::AnsiValue(n),
        _ => C::Reset,
    }
}

// ---- language ------------------------------------------------------------------

/// Turkish if KAZMA_LANG / `kz config lang` say so, or (auto) the Windows display
/// language / LANG is Turkish.
fn detect_turkish() -> bool {
    let pick = std::env::var("KAZMA_LANG")
        .ok()
        .map(|v| v.to_lowercase())
        .or_else(|| crate::settings::get("lang"));
    match pick.as_deref() {
        Some("tr") => true,
        Some("en") => false,
        _ => crate::sys::ui_language_is_turkish(),
    }
}

/// Plain text in the current language.
pub fn t(en: &'static str, tr: &'static str) -> &'static str {
    if is_tr() { tr } else { en }
}

pub fn is_tr() -> bool {
    *TURKISH.get_or_init(detect_turkish)
}

/// Load theme and language once at startup.
pub fn init() {
    let name = std::env::var("KAZMA_THEME")
        .ok()
        .or_else(|| crate::settings::get("theme"))
        .unwrap_or_default();
    let _ = THEME.set(theme_by_name(&name.to_lowercase()));
    let _ = TURKISH.set(detect_turkish());
}

/// Pick the English or Turkish text. Both arms are `format!` strings and can use
/// the same arguments: `tr!("{n} files", "{n} dosya")`.
#[macro_export]
macro_rules! tr {
    ($en:literal, $tr:literal $(, $arg:expr)* $(,)?) => {
        if $crate::ui::is_tr() { format!($tr $(, $arg)*) } else { format!($en $(, $arg)*) }
    };
}

pub const BANNER: &str = r"
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
";

/// Pure ASCII box drawing: +---+ / | |
pub const ASCII_BORDER: border::Set = border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

pub fn block(title: &str) -> Block<'_> {
    Block::bordered()
        .border_set(ASCII_BORDER)
        .border_style(Style::new().fg(fg()))
        .title(Span::styled(
            format!("[ {title} ]"),
            Style::new().fg(hi()).add_modifier(Modifier::BOLD),
        ))
}

pub fn human(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

/// `[#######.......]` style bar.
pub fn bar(ratio: f64, width: usize) -> String {
    let r = ratio.clamp(0.0, 1.0);
    let filled = (r * width as f64).round() as usize;
    format!("[{}{}]", "#".repeat(filled), ".".repeat(width - filled))
}

pub fn print_banner(subtitle: &str) {
    println!("{}", BANNER.with(ct(fg())));
    println!("  {}\n", subtitle.with(ct(dim())));
}

pub fn section(title: &str) {
    println!("\n{} {}", ">>".with(ct(hi())).bold(), title.bold());
}

pub fn ok(msg: &str) {
    println!("  {} {msg}", "[ok]".with(ct(fg())));
}

pub fn skip(msg: &str) {
    println!("  {} {}", "[--]".with(ct(dim())), msg.with(ct(dim())));
}

pub fn bad(msg: &str) {
    println!("  {} {msg}", "[xx]".with(ct(warn_c())).bold());
}

pub fn info(msg: &str) {
    println!("  {} {msg}", "[..]".with(ct(hi())));
}

pub fn warn(msg: &str) {
    println!("  {} {msg}", "[!!]".with(ct(warn_c())));
}

pub fn interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

pub fn pressed(ev: &Event) -> Option<KeyCode> {
    match ev {
        Event::Key(k) if k.kind == KeyEventKind::Press => Some(k.code),
        _ => None,
    }
}

pub struct CheckItem {
    pub label: String,
    pub detail: String,
    pub size: u64,
    pub checked: bool,
}

// ---- full-screen helpers ---------------------------------------------------

/// Enter the full-screen UI with mouse support.
pub fn tui_start() -> ratatui::DefaultTerminal {
    let t = ratatui::init();
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);
    t
}

/// Leave the full-screen UI.
pub fn tui_end() {
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    ratatui::restore();
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mouse {
    /// Left click on list row `index`, at column `col` inside the list.
    Click {
        index: usize,
        col: u16,
    },
    ScrollUp,
    ScrollDown,
}

/// Translate a mouse event over a bordered list drawn in `area` (with `offset`
/// rows scrolled away) into a row index or a scroll.
pub fn mouse(ev: &Event, area: Rect, offset: usize, len: usize) -> Option<Mouse> {
    use crossterm::event::{MouseButton, MouseEventKind};
    let Event::Mouse(m) = ev else { return None };
    let inside = m.column > area.x
        && m.column + 1 < area.x + area.width
        && m.row > area.y
        && m.row + 1 < area.y + area.height;
    match m.kind {
        MouseEventKind::ScrollUp if inside => Some(Mouse::ScrollUp),
        MouseEventKind::ScrollDown if inside => Some(Mouse::ScrollDown),
        MouseEventKind::Down(MouseButton::Left) if inside => {
            let index = offset + (m.row - area.y - 1) as usize;
            (index < len).then_some(Mouse::Click {
                index,
                col: m.column - area.x - 1,
            })
        }
        _ => None,
    }
}

/// `/` search box shared by the list screens.
#[derive(Default)]
pub struct Search {
    pub typing: bool,
    pub query: String,
}

impl Search {
    /// Handle a key; returns true when the key was used by the search box.
    pub fn key(&mut self, code: KeyCode) -> bool {
        if self.typing {
            match code {
                KeyCode::Enter => self.typing = false,
                KeyCode::Esc => {
                    self.typing = false;
                    self.query.clear();
                }
                KeyCode::Backspace => {
                    self.query.pop();
                }
                KeyCode::Char(c) => self.query.push(c),
                _ => return false,
            }
            return true;
        }
        if code == KeyCode::Char('/') {
            self.typing = true;
            return true;
        }
        if code == KeyCode::Esc && !self.query.is_empty() {
            self.query.clear();
            return true;
        }
        false
    }

    pub fn matches(&self, text: &str) -> bool {
        self.query.is_empty() || text.to_lowercase().contains(&self.query.to_lowercase())
    }

    /// Text for the footer: the box while typing / filtering, else nothing.
    pub fn status(&self) -> Option<String> {
        if self.typing {
            Some(format!(" /{}_", self.query))
        } else if !self.query.is_empty() {
            Some(crate::tr!(
                " filter: {}  [esc] clear",
                " filtre: {}  [esc] temizle",
                self.query
            ))
        } else {
            None
        }
    }
}

/// Full-screen checklist. Returns the checked flags, or None if cancelled.
pub fn checklist(title: &str, items: Vec<CheckItem>) -> anyhow::Result<Option<Vec<bool>>> {
    checklist_opts(title, items, true)
}

/// Checklist; `show_size` hides the size column for items without a size (programs, drivers).
pub fn checklist_opts(
    title: &str,
    mut items: Vec<CheckItem>,
    show_size: bool,
) -> anyhow::Result<Option<Vec<bool>>> {
    if items.is_empty() {
        return Ok(Some(vec![]));
    }
    let mut terminal = tui_start();
    let mut state = ListState::default().with_selected(Some(0));
    let mut search = Search::default();
    let mut list_area = Rect::default();
    let result = loop {
        let visible: Vec<usize> = (0..items.len())
            .filter(|&i| search.matches(&items[i].label) || search.matches(&items[i].detail))
            .collect();
        if state.selected().is_none_or(|s| s >= visible.len()) {
            state.select(
                (!visible.is_empty()).then_some(
                    visible
                        .len()
                        .saturating_sub(1)
                        .min(state.selected().unwrap_or(0)),
                ),
            );
        }
        let total: u64 = items.iter().filter(|i| i.checked).map(|i| i.size).sum();
        let count = items.iter().filter(|i| i.checked).count();
        terminal.draw(|f| {
            let [main, foot] =
                Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).areas(f.area());
            list_area = main;
            let w = main.width.saturating_sub(4) as usize;
            let rows: Vec<ListItem> = visible
                .iter()
                .map(|&idx| {
                    let i = &items[idx];
                    let mark = if i.checked { "[x]" } else { "[ ]" };
                    let size = if show_size {
                        format!("{:>10}", human(i.size))
                    } else {
                        String::new()
                    };
                    let label_w = w.saturating_sub(24).min(52);
                    let label = truncate(&i.label, label_w);
                    let detail_w = w.saturating_sub(label_w + 20);
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("{mark} "),
                            Style::new().fg(if i.checked { hi() } else { dim() }),
                        ),
                        Span::styled(format!("{label:<label_w$}"), Style::new().fg(fg())),
                        Span::styled(size, Style::new().fg(hi())),
                        Span::styled(
                            format!("  {}", truncate(&i.detail, detail_w)),
                            Style::new().fg(dim()),
                        ),
                    ]))
                })
                .collect();
            let shown = if visible.len() == items.len() {
                title.to_string()
            } else {
                format!("{title}  ({}/{})", visible.len(), items.len())
            };
            let list = List::new(rows)
                .block(block(&shown))
                .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
                .highlight_symbol(">");
            f.render_stateful_widget(list, main, &mut state);
            let help = match search.status() {
                Some(s) => Line::from(Span::styled(s, Style::new().fg(hi()).bold())),
                None => Line::from(vec![
                    Span::styled(
                        if show_size {
                            crate::tr!(
                                " selected: {count}  ->  {}   ",
                                " seçili: {count}  ->  {}   ",
                                human(total)
                            )
                        } else {
                            crate::tr!(" selected: {count}   ", " seçili: {count}   ")
                        },
                        Style::new().fg(hi()).bold(),
                    ),
                    Span::styled(
                        t(
                            "[space] toggle  [a] all  [n] none  [/] search  [enter] go  [q] cancel",
                            "[boşluk] seç  [a] hepsi  [n] hiçbiri  [/] ara  [enter] uygula  [q] iptal",
                        ),
                        Style::new().fg(dim()),
                    ),
                ]),
            };
            f.render_widget(Paragraph::new(help).block(block(t("keys", "tuşlar"))), foot);
        })?;
        let ev = event::read()?;
        let sel = state.selected().unwrap_or(0);
        if let Some(m) = mouse(&ev, list_area, state.offset(), visible.len()) {
            match m {
                Mouse::ScrollUp => state.select(Some(sel.saturating_sub(3))),
                Mouse::ScrollDown => {
                    state.select(Some((sel + 3).min(visible.len().saturating_sub(1))))
                }
                Mouse::Click { index, col } => {
                    // a click on the box, or on the row that is already selected, toggles it
                    if col <= 4 || index == sel {
                        items[visible[index]].checked = !items[visible[index]].checked;
                    }
                    state.select(Some(index));
                }
            }
            continue;
        }
        let Some(code) = pressed(&ev) else { continue };
        if search.key(code) {
            state.select(Some(0));
            continue;
        }
        let cur = visible.get(sel).copied();
        match code {
            KeyCode::Up | KeyCode::Char('k') => state.select(Some(sel.saturating_sub(1))),
            KeyCode::Down | KeyCode::Char('j') => {
                state.select(Some((sel + 1).min(visible.len().saturating_sub(1))))
            }
            KeyCode::PageUp => state.select(Some(sel.saturating_sub(15))),
            KeyCode::PageDown => {
                state.select(Some((sel + 15).min(visible.len().saturating_sub(1))))
            }
            KeyCode::Home => state.select(Some(0)),
            KeyCode::End => state.select(Some(visible.len().saturating_sub(1))),
            KeyCode::Char(' ') => {
                if let Some(i) = cur {
                    items[i].checked = !items[i].checked;
                }
            }
            KeyCode::Char('a') => visible.iter().for_each(|&i| items[i].checked = true),
            KeyCode::Char('n') => visible.iter().for_each(|&i| items[i].checked = false),
            KeyCode::Enter => break Some(items.iter().map(|i| i.checked).collect()),
            KeyCode::Char('q') | KeyCode::Esc => break None,
            _ => {}
        }
    };
    tui_end();
    Ok(result)
}

pub fn truncate(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        s.to_string()
    } else if max <= 3 {
        s.chars().take(max).collect()
    } else {
        let keep = max - 3;
        let tail: String = s.chars().skip(n - keep).collect();
        format!("...{tail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_box() {
        let mut s = Search::default();
        assert!(!s.key(KeyCode::Char('x')));
        assert!(s.key(KeyCode::Char('/')));
        for c in "ChRo".chars() {
            s.key(KeyCode::Char(c));
        }
        assert!(s.key(KeyCode::Enter));
        assert!(s.matches("Google Chrome cache") && !s.matches("Firefox"));
        assert!(s.key(KeyCode::Esc));
        assert!(s.matches("anything"));
    }

    #[test]
    fn mouse_rows() {
        use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
        let area = Rect {
            x: 0,
            y: 2,
            width: 40,
            height: 10,
        };
        let click = |col, row| {
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: col,
                row,
                modifiers: KeyModifiers::NONE,
            })
        };
        assert_eq!(
            mouse(&click(5, 3), area, 0, 5),
            Some(Mouse::Click { index: 0, col: 4 })
        );
        assert_eq!(
            mouse(&click(5, 5), area, 10, 20),
            Some(Mouse::Click { index: 12, col: 4 })
        );
        assert_eq!(mouse(&click(5, 2), area, 0, 5), None); // border
        assert_eq!(mouse(&click(5, 9), area, 0, 5), None); // past the last row
    }

    #[test]
    fn formatting() {
        assert_eq!(human(512), "512 B");
        assert_eq!(human(1536), "1.5 KB");
        assert_eq!(human(3 * 1024 * 1024 * 1024), "3.0 GB");
        assert_eq!(bar(0.5, 4), "[##..]");
        assert_eq!(truncate("abcdefgh", 6), "...fgh");
    }
}
