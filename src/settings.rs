//! Persistent settings in `%LOCALAPPDATA%\kazma\config.txt` (`key = value` lines),
//! plus `kz config` to view and change them.

use crate::{oplog, tr, ui};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const KEYS: [(&str, &str); 2] = [
    (
        "lang",
        "auto | en | tr | zh-CN | zh-TW | ja | ko | de | fr | pl | uk | ru",
    ),
    ("theme", "green | amber | white | blue"),
];

pub fn file() -> Option<PathBuf> {
    oplog::data_dir().map(|d| d.join("config.txt"))
}

pub fn load() -> BTreeMap<String, String> {
    let Some(f) = file() else {
        return BTreeMap::new();
    };
    std::fs::read_to_string(f)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let l = l.split('#').next()?.trim();
            let (k, v) = l.split_once('=')?;
            Some((k.trim().to_lowercase(), v.trim().to_lowercase()))
        })
        .collect()
}

pub fn get(key: &str) -> Option<String> {
    load().remove(key)
}

fn save(map: &BTreeMap<String, String>) -> anyhow::Result<PathBuf> {
    let f = file().ok_or_else(|| anyhow::anyhow!("no settings folder"))?;
    if let Some(d) = f.parent() {
        std::fs::create_dir_all(d)?;
    }
    let body: String = map.iter().map(|(k, v)| format!("{k} = {v}\n")).collect();
    std::fs::write(&f, format!("# kazma settings (kz config)\n{body}"))?;
    Ok(f)
}

fn valid(key: &str, value: &str) -> bool {
    match key {
        "lang" => value == "auto" || crate::i18n::parse(value).is_some(),
        "theme" => matches!(value, "green" | "amber" | "white" | "blue"),
        _ => false,
    }
}

pub fn run(key: Option<String>, value: Option<String>) -> anyhow::Result<()> {
    if key.is_none() && ui::interactive() {
        return pick();
    }
    match (key, value) {
        (Some(k), Some(v)) => {
            let (k, v) = (k.to_lowercase(), v.to_lowercase());
            if !valid(&k, &v) {
                let hint = KEYS
                    .iter()
                    .find(|(n, _)| *n == k)
                    .map(|(_, h)| *h)
                    .unwrap_or("lang, theme");
                anyhow::bail!(
                    "{}",
                    tr!("invalid value. {k}: {hint}", "geçersiz değer. {k}: {hint}")
                );
            }
            let mut m = load();
            m.insert(k.clone(), v.clone());
            let f = save(&m)?;
            ui::ok(&tr!(
                "{k} = {v}  (saved to {})",
                "{k} = {v}  ({} dosyasına kaydedildi)",
                f.display()
            ));
        }
        (Some(k), None) => println!(
            "{}",
            get(&k.to_lowercase()).unwrap_or_else(|| "auto".into())
        ),
        _ => {
            ui::print_banner(&tr!("config :: settings", "config :: ayarlar"));
            let m = load();
            for (k, hint) in KEYS {
                let v = m.get(k).cloned().unwrap_or_else(|| "auto".into());
                println!("  {k:<8} = {}  ({hint})", ui::pad(&v, 8));
            }
            println!();
            println!(
                "  {}",
                tr!(
                    "change:  kz config lang tr    kz config theme amber",
                    "değiştir:  kz config lang en    kz config theme amber"
                )
            );
            if let Some(f) = file() {
                println!("  {}\n", f.display());
            }
        }
    }
    Ok(())
}

// ---- interactive screen ----------------------------------------------------------

enum Row {
    Header(&'static str),
    /// None = auto (follow Windows)
    Lang(Option<crate::i18n::Lang>),
    Theme(&'static str),
}

fn rows() -> Vec<Row> {
    let mut v = vec![Row::Header("lang"), Row::Lang(None)];
    v.extend(crate::i18n::LANGS.iter().map(|l| Row::Lang(Some(l.0))));
    v.push(Row::Header("theme"));
    v.extend(ui::THEMES.iter().map(|t| Row::Theme(t)));
    v
}

fn lang_name(l: crate::i18n::Lang) -> &'static str {
    crate::i18n::LANGS
        .iter()
        .find(|x| x.0 == l)
        .map_or("", |x| x.2)
}

fn lang_code(l: crate::i18n::Lang) -> &'static str {
    crate::i18n::LANGS
        .iter()
        .find(|x| x.0 == l)
        .map_or("", |x| x.1)
}

/// `kz config` in a terminal: pick the language and theme with the arrow keys.
/// A choice is saved and applied at once, so the screen redraws in it.
fn pick() -> anyhow::Result<()> {
    use crossterm::event::{self, KeyCode};
    use ratatui::layout::{Constraint, Layout, Rect};
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{List, ListItem, ListState, Paragraph};

    let rows = rows();
    let selectable = |i: usize| !matches!(rows[i], Row::Header(_));
    let mut state = ListState::default().with_selected(Some(1));
    let mut terminal = ui::tui_start();
    let mut area = Rect::default();
    let mut note = String::new();
    let res = (|| -> anyhow::Result<()> {
        loop {
            let m = load();
            let cur_lang = m.get("lang").cloned().unwrap_or_else(|| "auto".into());
            let cur_theme = m.get("theme").cloned().unwrap_or_else(|| "green".into());
            terminal.draw(|f| {
                let [main, foot] =
                    Layout::vertical([Constraint::Min(5), Constraint::Length(4)]).areas(f.area());
                area = main;
                let items: Vec<ListItem> = rows
                    .iter()
                    .map(|r| match r {
                        Row::Header(h) => {
                            let title = if *h == "lang" {
                                ui::t("LANGUAGE", "DİL")
                            } else {
                                ui::t("COLOR THEME", "RENK TEMASI")
                            };
                            ListItem::new(Line::from(Span::styled(
                                format!(" {title}"),
                                Style::new().fg(ui::hi()).add_modifier(Modifier::BOLD),
                            )))
                        }
                        Row::Lang(l) => {
                            let on = match l {
                                None => cur_lang == "auto",
                                Some(l) => crate::i18n::parse(&cur_lang) == Some(*l),
                            };
                            let (name, detail) = match l {
                                None => (
                                    ui::t("automatic", "otomatik").to_string(),
                                    tr!(
                                        "follows Windows (now: {})",
                                        "Windows'u izler (şu an: {})",
                                        lang_name(crate::i18n::auto())
                                    ),
                                ),
                                Some(l) => (lang_name(*l).to_string(), lang_code(*l).to_string()),
                            };
                            option(
                                on,
                                &name,
                                vec![Span::styled(detail, Style::new().fg(ui::dim()))],
                            )
                        }
                        Row::Theme(t) => {
                            let th = ui::theme_by_name(t);
                            let mut sample: Vec<Span> = th
                                .grad
                                .iter()
                                .map(|c| {
                                    Span::styled(
                                        "##",
                                        Style::new().fg(ratatui::style::Color::Indexed(*c)),
                                    )
                                })
                                .collect();
                            sample.push(Span::styled(
                                "  kazma",
                                Style::new().fg(th.fg).add_modifier(Modifier::BOLD),
                            ));
                            sample.push(Span::styled(" / clean", Style::new().fg(th.hi)));
                            option(cur_theme == *t, t, sample)
                        }
                    })
                    .collect();
                let list = List::new(items)
                    .block(ui::block(ui::t(
                        "kazma config :: settings",
                        "kazma config :: ayarlar",
                    )))
                    .highlight_style(ui::sel_style())
                    .highlight_symbol(">");
                f.render_stateful_widget(list, main, &mut state);
                let first = if note.is_empty() {
                    tr!(
                        "choices are saved at once to {}",
                        "seçimler hemen {} dosyasına kaydedilir",
                        file().map(|p| p.display().to_string()).unwrap_or_default()
                    )
                } else {
                    note.clone()
                };
                let help = Paragraph::new(vec![
                    Line::from(Span::styled(format!(" {first}"), Style::new().fg(ui::fg()))),
                    ui::keys(&[
                        ("up/down", ui::t("move", "gez")),
                        ("enter", ui::t("choose", "seç")),
                        ("q", ui::t("done", "bitti")),
                    ]),
                ])
                .block(ui::block(ui::t("keys", "tuşlar")));
                f.render_widget(help, foot);
            })?;

            let ev = event::read()?;
            let mut choose = None;
            if let Some(ui::Mouse::Click { index, .. }) =
                ui::mouse(&ev, area, state.offset(), rows.len())
                && selectable(index)
            {
                state.select(Some(index));
                choose = Some(index);
            }
            if let Some(code) = ui::pressed(&ev) {
                let sel = state.selected().unwrap_or(1);
                match code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if let Some(i) = (0..sel).rev().find(|&i| selectable(i)) {
                            state.select(Some(i));
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if let Some(i) = (sel + 1..rows.len()).find(|&i| selectable(i)) {
                            state.select(Some(i));
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => choose = Some(sel),
                    KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                    _ => {}
                }
            }
            if let Some(i) = choose {
                let mut m = load();
                match &rows[i] {
                    Row::Lang(l) => {
                        let v = l.map_or("auto", lang_code);
                        m.insert("lang".into(), v.to_lowercase());
                        save(&m)?;
                        crate::i18n::reload();
                        note = tr!(
                            "saved: language = {}",
                            "kaydedildi: dil = {}",
                            l.map_or(ui::t("automatic", "otomatik"), lang_name)
                        );
                    }
                    Row::Theme(t) => {
                        m.insert("theme".into(), t.to_string());
                        save(&m)?;
                        ui::set_theme(t);
                        note = tr!("saved: theme = {t}", "kaydedildi: tema = {t}");
                    }
                    Row::Header(_) => {}
                }
            }
        }
    })();
    ui::tui_end();
    res
}

fn option(
    on: bool,
    name: &str,
    detail: Vec<ratatui::text::Span<'static>>,
) -> ratatui::widgets::ListItem<'static> {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    let (mark, c) = if on {
        ("(*)", ui::hi())
    } else {
        ("( )", ui::line_c())
    };
    let mut spans = vec![
        Span::styled(
            format!("   {mark} "),
            Style::new().fg(c).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            ui::pad(name, 14),
            if on {
                Style::new().fg(ui::fg()).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(ui::fg())
            },
        ),
        Span::raw("  "),
    ];
    spans.extend(detail);
    ratatui::widgets::ListItem::new(Line::from(spans))
}
