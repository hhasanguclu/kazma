//! Retro ASCII look: banner, borders, colors, size formatting and a checklist widget.

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::style::Stylize;
use ratatui::layout::{Constraint, Layout};
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
    let mut terminal = ratatui::init();
    let mut state = ListState::default().with_selected(Some(0));
    let result = loop {
        let total: u64 = items.iter().filter(|i| i.checked).map(|i| i.size).sum();
        let count = items.iter().filter(|i| i.checked).count();
        terminal.draw(|f| {
            let [main, foot] =
                Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).areas(f.area());
            let w = main.width.saturating_sub(4) as usize;
            let rows: Vec<ListItem> = items
                .iter()
                .map(|i| {
                    let mark = if i.checked { "[x]" } else { "[ ]" };
                    let size = if show_size {
                        format!("{:>10}", human(i.size))
                    } else {
                        String::new()
                    };
                    let label_w = w.saturating_sub(20).min(40);
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
            let list = List::new(rows)
                .block(block(title))
                .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
                .highlight_symbol(">");
            f.render_stateful_widget(list, main, &mut state);
            let help = Line::from(vec![
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
                        "[space] toggle  [a] all  [n] none  [enter] go  [q] cancel",
                        "[boşluk] seç  [a] hepsi  [n] hiçbiri  [enter] uygula  [q] iptal",
                    ),
                    Style::new().fg(dim()),
                ),
            ]);
            f.render_widget(Paragraph::new(help).block(block(t("keys", "tuşlar"))), foot);
        })?;
        let ev = event::read()?;
        let Some(code) = pressed(&ev) else { continue };
        let sel = state.selected().unwrap_or(0);
        match code {
            KeyCode::Up | KeyCode::Char('k') => state.select(Some(sel.saturating_sub(1))),
            KeyCode::Down | KeyCode::Char('j') => {
                state.select(Some((sel + 1).min(items.len() - 1)))
            }
            KeyCode::Home => state.select(Some(0)),
            KeyCode::End => state.select(Some(items.len() - 1)),
            KeyCode::Char(' ') => items[sel].checked = !items[sel].checked,
            KeyCode::Char('a') => items.iter_mut().for_each(|i| i.checked = true),
            KeyCode::Char('n') => items.iter_mut().for_each(|i| i.checked = false),
            KeyCode::Enter => break Some(items.iter().map(|i| i.checked).collect()),
            KeyCode::Char('q') | KeyCode::Esc => break None,
            _ => {}
        }
    };
    ratatui::restore();
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
    fn formatting() {
        assert_eq!(human(512), "512 B");
        assert_eq!(human(1536), "1.5 KB");
        assert_eq!(human(3 * 1024 * 1024 * 1024), "3.0 GB");
        assert_eq!(bar(0.5, 4), "[##..]");
        assert_eq!(truncate("abcdefgh", 6), "...fgh");
    }
}
