//! Retro ASCII look: banner, borders, colors, size formatting and a checklist widget.

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::style::Stylize;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};
use std::io::IsTerminal;
use std::sync::RwLock;

// ---- theme -------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct Theme {
    /// Main text.
    pub fg: Color,
    /// Accent: titles, keys, numbers.
    pub hi: Color,
    /// Secondary text.
    pub dim: Color,
    pub warn: Color,
    /// Box lines: quieter than text so the content stands out.
    pub line: Color,
    /// Background of the selected row.
    pub sel: Color,
    /// Banner gradient, top to bottom.
    pub grad: [u8; 5],
}

pub fn theme_by_name(name: &str) -> Theme {
    match name {
        "amber" => Theme {
            fg: Color::Indexed(214),
            hi: Color::Indexed(229),
            dim: Color::Indexed(137),
            warn: Color::Indexed(196),
            line: Color::Indexed(94),
            sel: Color::Indexed(58),
            grad: [229, 221, 214, 208, 166],
        },
        "white" => Theme {
            fg: Color::Indexed(252),
            hi: Color::Indexed(231),
            dim: Color::Indexed(244),
            warn: Color::Indexed(203),
            line: Color::Indexed(240),
            sel: Color::Indexed(238),
            grad: [231, 254, 251, 248, 245],
        },
        "blue" => Theme {
            fg: Color::Indexed(117),
            hi: Color::Indexed(229),
            dim: Color::Indexed(67),
            warn: Color::Indexed(203),
            line: Color::Indexed(25),
            sel: Color::Indexed(17),
            grad: [159, 123, 117, 75, 33],
        },
        _ => Theme {
            fg: Color::Indexed(77),
            hi: Color::Indexed(221),
            dim: Color::Indexed(244),
            warn: Color::Indexed(203),
            line: Color::Indexed(28),
            sel: Color::Indexed(22),
            grad: [120, 84, 77, 71, 35],
        },
    }
}

/// The color themes, in the order `kz config` lists them.
pub const THEMES: [&str; 4] = ["green", "amber", "white", "blue"];

static THEME: RwLock<Option<Theme>> = RwLock::new(None);

fn theme() -> Theme {
    THEME
        .read()
        .ok()
        .and_then(|t| *t)
        .unwrap_or_else(|| theme_by_name("green"))
}

/// Switch the color theme (used at startup and live by `kz config`).
pub fn set_theme(name: &str) {
    if let Ok(mut t) = THEME.write() {
        *t = Some(theme_by_name(&name.to_lowercase()));
    }
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
pub fn line_c() -> Color {
    theme().line
}
pub fn grad(i: usize) -> Color {
    let g = theme().grad;
    Color::Indexed(g[i.min(g.len() - 1)])
}

/// Style of the selected row: one calm background bar, text keeps its colors.
pub fn sel_style() -> Style {
    Style::new().bg(theme().sel).add_modifier(Modifier::BOLD)
}

/// `[######......]` as colored spans: filled part in `color`, the rest dim.
pub fn bar_spans(ratio: f64, width: usize, color: Color) -> Vec<Span<'static>> {
    let filled = (ratio.clamp(0.0, 1.0) * width as f64).round() as usize;
    vec![
        Span::styled("[", Style::new().fg(line_c())),
        Span::styled("#".repeat(filled), Style::new().fg(color)),
        Span::styled(".".repeat(width - filled), Style::new().fg(line_c())),
        Span::styled("]", Style::new().fg(line_c())),
    ]
}

/// Footer key hints: keys in the accent color, what they do in dim.
pub fn keys(pairs: &[(&str, &str)]) -> Line<'static> {
    let mut spans = vec![Span::raw(" ")];
    for (k, label) in pairs {
        spans.push(Span::styled(
            k.to_string(),
            Style::new().fg(hi()).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(format!(" {label}   "), Style::new().fg(dim())));
    }
    Line::from(spans)
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

/// Plain text in the current language.
pub fn t(en: &'static str, tr: &'static str) -> &'static str {
    match crate::i18n::lang() {
        crate::i18n::Lang::En => en,
        crate::i18n::Lang::Tr => tr,
        _ => crate::i18n::get(en).unwrap_or(en),
    }
}

pub fn is_tr() -> bool {
    crate::i18n::lang() == crate::i18n::Lang::Tr
}

/// Load theme and language once at startup.
pub fn init() {
    let name = std::env::var("KAZMA_THEME")
        .ok()
        .or_else(|| crate::settings::get("theme"))
        .unwrap_or_default();
    set_theme(&name);
    crate::i18n::lang();
}

pub const BANNER: &str = r"
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
";

/// Pure ASCII box drawing with soft corners:  .----.  |    |  '----'
pub const ASCII_BORDER: border::Set = border::Set {
    top_left: ".",
    top_right: ".",
    bottom_left: "'",
    bottom_right: "'",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

pub fn block(title: &str) -> Block<'_> {
    Block::bordered()
        .border_set(ASCII_BORDER)
        .border_style(Style::new().fg(line_c()))
        .title(Line::from(vec![
            Span::styled("[ ", Style::new().fg(line_c())),
            Span::styled(title, Style::new().fg(hi()).add_modifier(Modifier::BOLD)),
            Span::styled(" ]", Style::new().fg(line_c())),
        ]))
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

/// `[#####.....]` for printed (non full-screen) output: filled part in the
/// main color, the rest in the quiet line color.
pub fn cbar(ratio: f64, width: usize) -> String {
    let filled = (ratio.clamp(0.0, 1.0) * width as f64).round() as usize;
    let l = ct(line_c());
    format!(
        "{}{}{}{}",
        "[".with(l),
        "#".repeat(filled).with(ct(fg())),
        ".".repeat(width - filled).with(l),
        "]".with(l)
    )
}

/// `[#######.......]` style bar (plain text).
#[cfg(test)]
pub fn bar(ratio: f64, width: usize) -> String {
    let r = ratio.clamp(0.0, 1.0);
    let filled = (r * width as f64).round() as usize;
    format!("[{}{}]", "#".repeat(filled), ".".repeat(width - filled))
}

/// Compact header for command output:
///   .-----------------------------------------------------------------.
///   | kazma / clean   reclaim disk space                       v0.5.0 |
///   '-----------------------------------------------------------------'
pub fn print_banner(subtitle: &str) {
    const W: usize = 66;
    let (cmd, desc) = subtitle.split_once(" :: ").unwrap_or(("", subtitle));
    let ver = format!("v{}", env!("CARGO_PKG_VERSION"));
    let used = "kazma".len() + 3 + width(cmd) + 3 + ver.len();
    let room = W.saturating_sub(used + 2);
    let desc = truncate_end(desc, room);
    let pad = room.saturating_sub(width(&desc));
    let l = ct(line_c());
    println!();
    println!("  {}", format!(".{}.", "-".repeat(W)).with(l));
    println!(
        "  {} {}{}{}   {}{}{} {}",
        "|".with(l),
        "kazma".with(ct(fg())).bold(),
        " / ".with(l),
        cmd.with(ct(hi())).bold(),
        desc.with(ct(fg())),
        " ".repeat(pad),
        ver.with(ct(dim())),
        "|".with(l)
    );
    println!("  {}", format!("'{}'", "-".repeat(W)).with(l));
    println!();
}

/// Section heading:  Browser ----------------------------------
pub fn section(title: &str) {
    let rule = 62usize.saturating_sub(width(title));
    println!(
        "\n  {} {}",
        title.with(ct(hi())).bold(),
        "-".repeat(rule).with(ct(line_c()))
    );
}

/// Cut from the end ("long text...") for headers; `truncate` keeps the end for paths.
/// Terminal columns a text takes (CJK characters take two).
pub fn width(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(s)
}

fn cw(c: char) -> usize {
    unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
}

/// Cut to at most `max` columns, keeping the start: "long name..."
pub fn truncate_end(s: &str, max: usize) -> String {
    if width(s) <= max {
        return s.to_string();
    }
    let room = max.saturating_sub(if max > 3 { 3 } else { 0 });
    let (mut out, mut w) = (String::new(), 0);
    for c in s.chars() {
        if w + cw(c) > room {
            break;
        }
        w += cw(c);
        out.push(c);
    }
    if max > 3 {
        out.push_str("...");
    }
    out
}

/// Pad with spaces to at least `n` columns (like `{:<n}`, but CJK-aware).
pub fn pad(s: &str, n: usize) -> String {
    format!("{s}{}", " ".repeat(n.saturating_sub(width(s))))
}

/// Break text into lines of at most `max` columns (CJK-aware), at spaces when it can.
pub fn wrap(text: &str, max: usize) -> Vec<String> {
    let max = max.max(4);
    let mut out = vec![];
    let mut line = String::new();
    for word in text.split_inclusive(' ') {
        if width(&line) + width(word.trim_end()) > max && !line.is_empty() {
            out.push(line.trim_end().to_string());
            line.clear();
        }
        // a word longer than a line: cut it
        let mut w = word;
        while width(w.trim_end()) > max {
            let cut = truncate_end_raw(w, max);
            out.push(cut.to_string());
            w = &w[cut.len()..];
        }
        line.push_str(w);
    }
    if !line.trim().is_empty() || out.is_empty() {
        out.push(line.trim_end().to_string());
    }
    out
}

/// The longest prefix of `s` that fits in `max` columns.
fn truncate_end_raw(s: &str, max: usize) -> &str {
    let mut w = 0;
    for (i, c) in s.char_indices() {
        w += cw(c);
        if w > max {
            return &s[..i];
        }
    }
    s
}

/// Cut to `max` columns and pad with spaces to exactly `max` columns.
pub fn fit(s: &str, max: usize) -> String {
    let t = truncate_end(s, max);
    let pad = max.saturating_sub(width(&t));
    format!("{t}{}", " ".repeat(pad))
}

pub fn ok(msg: &str) {
    println!("  {} {msg}", "[ok]".with(ct(fg())));
}

pub fn skip(msg: &str) {
    println!("  {} {}", "[--]".with(ct(line_c())), msg.with(ct(dim())));
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

/// Columns of the terminal; 120 when there is none (output to a file or pipe).
pub fn term_width() -> usize {
    crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(120)
        .clamp(60, 240)
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
                Layout::vertical([Constraint::Min(3), Constraint::Length(4)]).areas(f.area());
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
                    // checked rows are bright, unchecked rows step back
                    let (mark_c, text_c) = if i.checked {
                        (hi(), fg())
                    } else {
                        (line_c(), dim())
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!(" {mark} "),
                            Style::new().fg(mark_c).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(pad(&label, label_w), Style::new().fg(text_c)),
                        Span::styled(size, Style::new().fg(if i.checked { hi() } else { dim() })),
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
                .highlight_style(sel_style())
                .highlight_symbol(">");
            f.render_stateful_widget(list, main, &mut state);
            let mut summary = vec![
                Span::raw(" "),
                Span::styled(
                    crate::tr!(
                        "{count} of {} selected",
                        "{} öğeden {count} seçili",
                        items.len()
                    ),
                    Style::new().fg(fg()).add_modifier(Modifier::BOLD),
                ),
            ];
            if show_size {
                summary.push(Span::styled("   =   ", Style::new().fg(line_c())));
                summary.push(Span::styled(
                    human(total),
                    Style::new().fg(hi()).add_modifier(Modifier::BOLD),
                ));
            }
            let second = match search.status() {
                Some(st) => Line::from(Span::styled(st, Style::new().fg(hi()).bold())),
                None => keys(&[
                    (t("space", "boşluk"), t("toggle", "seç")),
                    ("a", t("all", "hepsi")),
                    ("n", t("none", "hiçbiri")),
                    ("/", t("search", "ara")),
                    ("enter", t("go", "uygula")),
                    ("q", t("cancel", "iptal")),
                ]),
            };
            f.render_widget(
                Paragraph::new(vec![Line::from(summary), second]).block(block(t("keys", "tuşlar"))),
                foot,
            );
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

/// Cut to at most `max` columns, keeping the end: "...\\long\\path"
pub fn truncate(s: &str, max: usize) -> String {
    if width(s) <= max {
        return s.to_string();
    }
    let room = max.saturating_sub(if max > 3 { 3 } else { 0 });
    let (mut tail, mut w) = (Vec::new(), 0);
    for c in s.chars().rev() {
        if w + cw(c) > room {
            break;
        }
        w += cw(c);
        tail.push(c);
    }
    let tail: String = tail.into_iter().rev().collect();
    if max > 3 { format!("...{tail}") } else { tail }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_by_display_width() {
        assert_eq!(wrap("aaa bbb ccc", 7), vec!["aaa bbb", "ccc"]);
        assert_eq!(wrap("/very/long/path/without/spaces", 10).len(), 3);
        for l in wrap("中文字符很长的一段文本没有空格", 9) {
            assert!(width(&l) <= 9, "{l}");
        }
        assert_eq!(wrap("", 10), vec![""]);
    }

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
