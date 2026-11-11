//! `kz awake`: keep the screen on (no sleep, no screen saver) for a while.
//! `kz cleanscreen`: a black, input-ignoring screen so you can wipe the display.

use crate::{tr, ui};
use crossterm::event::{self, KeyCode};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use std::time::{Duration, Instant};

fn clock(secs: u64) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

fn center(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

pub fn awake(minutes: u64) -> anyhow::Result<()> {
    if !cfg!(windows) {
        ui::print_banner(ui::t(
            "awake :: keep the screen on",
            "awake :: ekranı açık tut",
        ));
        ui::warn(ui::t(
            "kz awake works on Windows only.",
            "kz awake yalnızca Windows'ta çalışır.",
        ));
        return Ok(());
    }
    let until = (minutes > 0).then(|| Instant::now() + Duration::from_secs(minutes * 60));
    let started = Instant::now();
    crate::gpu::keep_awake(true);
    let mut terminal = ui::tui_start();
    let res = (|| -> anyhow::Result<()> {
        loop {
            if until.is_some_and(|u| Instant::now() >= u) {
                return Ok(());
            }
            terminal.draw(|f| {
                let r = center(f.area(), 52, 7);
                let status = match until {
                    Some(u) => tr!(
                        "turns off in {}",
                        "{} sonra kapanır",
                        clock(u.saturating_duration_since(Instant::now()).as_secs())
                    ),
                    None => tr!(
                        "on for {}",
                        "{} süredir açık",
                        clock(started.elapsed().as_secs())
                    ),
                };
                let lines = vec![
                    Line::default(),
                    Line::from(Span::styled(
                        ui::t(
                            "  screen and PC stay awake",
                            "  ekran ve bilgisayar uyanık kalıyor",
                        ),
                        Style::new().fg(ui::fg()).bold(),
                    )),
                    Line::from(Span::styled(
                        format!("  {status}"),
                        Style::new().fg(ui::hi()),
                    )),
                    Line::default(),
                    ui::keys(&[("q", ui::t("stop", "durdur"))]),
                ];
                f.render_widget(Paragraph::new(lines).block(ui::block("kazma awake")), r);
            })?;
            if event::poll(Duration::from_millis(500))?
                && let Some(KeyCode::Char('q') | KeyCode::Esc) = ui::pressed(&event::read()?)
            {
                return Ok(());
            }
        }
    })();
    ui::tui_end();
    crate::gpu::keep_awake(false);
    res
}

pub fn cleanscreen(seconds: u64) -> anyhow::Result<()> {
    let end = Instant::now() + Duration::from_secs(seconds.clamp(5, 600));
    let mut terminal = ui::tui_start();
    let mut esc_hits: Vec<Instant> = vec![];
    let res = (|| -> anyhow::Result<()> {
        loop {
            let left = end.saturating_duration_since(Instant::now()).as_secs();
            if left == 0 {
                return Ok(());
            }
            terminal.draw(|f| {
                let area = f.area();
                f.render_widget(Block::new().style(Style::new().bg(Color::Black)), area);
                // a tiny, dark hint in the corner; everything else is black
                let [_, bottom] =
                    Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);
                let hint = tr!(
                    " {left}s  -  esc x3 to stop ",
                    " {left} sn  -  durdurmak için 3 kez esc "
                );
                f.render_widget(
                    Paragraph::new(Span::styled(
                        hint,
                        Style::new().fg(Color::Indexed(236)).bg(Color::Black),
                    ))
                    .right_aligned(),
                    bottom,
                );
            })?;
            if event::poll(Duration::from_millis(250))? {
                // every key and click is swallowed while you wipe; three Esc in 2 s stop it
                if let Some(KeyCode::Esc) = ui::pressed(&event::read()?) {
                    esc_hits.push(Instant::now());
                    esc_hits.retain(|t| t.elapsed() < Duration::from_secs(2));
                    if esc_hits.len() >= 3 {
                        return Ok(());
                    }
                }
            }
        }
    })();
    ui::tui_end();
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers() {
        assert_eq!(clock(3725), "01:02:05");
        let r = center(
            Rect {
                x: 0,
                y: 0,
                width: 100,
                height: 30,
            },
            52,
            7,
        );
        assert_eq!((r.x, r.y, r.width, r.height), (24, 11, 52, 7));
    }
}
