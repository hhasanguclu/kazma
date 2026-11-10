//! `kz status`: live system dashboard.

use crate::ui;
use crossterm::event::{self, KeyCode};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, ProcessesToUpdate, System};

const HISTORY: usize = 120;

fn level_color(pct: f64) -> Color {
    if pct >= 85.0 {
        ui::warn_c()
    } else if pct >= 60.0 {
        ui::hi()
    } else {
        ui::fg()
    }
}

fn meter(label: &str, pct: f64, extra: String, width: u16) -> Line<'static> {
    let bar_w = (width as usize)
        .saturating_sub(label.len() + extra.len() + 12)
        .clamp(5, 60);
    Line::from(vec![
        Span::styled(format!("{label} "), Style::new().fg(ui::fg())),
        Span::styled(
            ui::bar(pct / 100.0, bar_w),
            Style::new().fg(level_color(pct)),
        ),
        Span::styled(format!(" {pct:>5.1}% "), Style::new().fg(ui::hi())),
        Span::styled(extra, Style::new().fg(ui::dim())),
    ])
}

/// Tiny ASCII sparkline using " .:-=+*#%@" density ramp.
fn spark(data: &VecDeque<f64>, width: usize, max: f64) -> String {
    const RAMP: &[u8] = b" .:-=+*#%@";
    let start = data.len().saturating_sub(width);
    let mut s: String = data
        .iter()
        .skip(start)
        .map(|v| {
            let idx =
                ((v / max.max(1e-9)).clamp(0.0, 1.0) * (RAMP.len() - 1) as f64).round() as usize;
            RAMP[idx] as char
        })
        .collect();
    while s.len() < width {
        s.insert(0, ' ');
    }
    s
}

fn uptime(secs: u64) -> String {
    format!(
        "{}d {:02}h {:02}m",
        secs / 86_400,
        secs % 86_400 / 3600,
        secs % 3600 / 60
    )
}

pub fn run() -> anyhow::Result<()> {
    let mut sys = System::new_all()?;
    let mut disks = Disks::new_with_refreshed_list()?;
    let mut nets = Networks::new_with_refreshed_list()?;
    let mut cpu_hist: VecDeque<f64> = VecDeque::with_capacity(HISTORY);
    let mut rx_hist: VecDeque<f64> = VecDeque::with_capacity(HISTORY);
    let mut tx_hist: VecDeque<f64> = VecDeque::with_capacity(HISTORY);
    let mut last = Instant::now();
    let mut tick = 0u64;
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);

    let host = System::host_name().unwrap_or_default();
    let os = System::long_os_version().unwrap_or_default();

    let mut terminal = ratatui::init();
    let res = (|| -> anyhow::Result<()> {
        loop {
            let dt = last.elapsed().as_secs_f64().max(0.001);
            last = Instant::now();
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            sys.refresh_processes(ProcessesToUpdate::All, true);
            nets.refresh(true);
            if tick.is_multiple_of(5) {
                disks.refresh(true);
            }
            tick += 1;

            let (rx, tx) = nets.iter().fold((0u64, 0u64), |(r, t), (_, d)| {
                (r + d.received(), t + d.transmitted())
            });
            let (rx_s, tx_s) = (rx as f64 / dt, tx as f64 / dt);
            for (h, v) in [
                (&mut cpu_hist, sys.global_cpu_usage() as f64),
                (&mut rx_hist, rx_s),
                (&mut tx_hist, tx_s),
            ] {
                if h.len() == HISTORY {
                    h.pop_front();
                }
                h.push_back(v);
            }

            terminal.draw(|f| {
                let cpus = sys.cpus();
                let core_rows = cpus.len().div_ceil(2) as u16;
                let [head, cpu, mem, mid, procs, foot] = Layout::vertical([
                    Constraint::Length(3),
                    Constraint::Length(core_rows + 4),
                    Constraint::Length(4),
                    Constraint::Length((disks.len() as u16).max(1) + 2),
                    Constraint::Min(5),
                    Constraint::Length(1),
                ])
                .areas(f.area());

                let header = Line::from(vec![
                    Span::styled(format!(" {host} "), Style::new().fg(ui::hi()).bold()),
                    Span::styled(format!(" {os} "), Style::new().fg(ui::fg())),
                    Span::styled(
                        crate::tr!(" up {}", " açık {}", uptime(System::uptime().unwrap_or(0))),
                        Style::new().fg(ui::dim()),
                    ),
                ]);
                f.render_widget(
                    Paragraph::new(header).block(ui::block("kazma status")),
                    head,
                );

                // CPU
                let inner_w = cpu.width.saturating_sub(4);
                let g = sys.global_cpu_usage() as f64;
                let mut lines = vec![
                    meter(
                        "CPU",
                        g,
                        crate::tr!("{} cores", "{} çekirdek", cpus.len()),
                        inner_w,
                    ),
                    Line::from(Span::styled(
                        format!(
                            "    {}",
                            spark(&cpu_hist, inner_w.saturating_sub(4) as usize, 100.0)
                        ),
                        Style::new().fg(ui::fg()),
                    )),
                ];
                let half = inner_w / 2;
                for pair in cpus.chunks(2).enumerate() {
                    let (row, chunk) = pair;
                    let mut spans = vec![];
                    for (j, c) in chunk.iter().enumerate() {
                        let idx = row * 2 + j;
                        let m = meter(
                            &format!("{idx:>3}"),
                            c.usage() as f64,
                            String::new(),
                            half.saturating_sub(1),
                        );
                        spans.extend(m.spans);
                        spans.push(Span::raw(" "));
                    }
                    lines.push(Line::from(spans));
                }
                f.render_widget(Paragraph::new(lines).block(ui::block("cpu")), cpu);

                // Memory + network
                let [mem_l, net_r] =
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .areas(mem);
                let (tm, um) = (sys.total_memory(), sys.used_memory());
                let (ts, us) = (sys.total_swap(), sys.used_swap());
                let pct = |u: u64, t: u64| {
                    if t == 0 {
                        0.0
                    } else {
                        u as f64 * 100.0 / t as f64
                    }
                };
                let w = mem_l.width.saturating_sub(4);
                f.render_widget(
                    Paragraph::new(vec![
                        meter(
                            "RAM ",
                            pct(um, tm),
                            format!("{}/{}", ui::human(um), ui::human(tm)),
                            w,
                        ),
                        meter(
                            "SWAP",
                            pct(us, ts),
                            format!("{}/{}", ui::human(us), ui::human(ts)),
                            w,
                        ),
                    ])
                    .block(ui::block(ui::t("memory", "bellek"))),
                    mem_l,
                );
                let nw = net_r.width.saturating_sub(4) as usize;
                let net_max = rx_hist
                    .iter()
                    .chain(tx_hist.iter())
                    .cloned()
                    .fold(1.0, f64::max);
                let sw = nw.saturating_sub(18);
                f.render_widget(
                    Paragraph::new(vec![
                        Line::from(vec![
                            Span::styled(
                                crate::tr!(
                                    "down {:>10}/s ",
                                    "indir {:>10}/s ",
                                    ui::human(rx_s as u64)
                                ),
                                Style::new().fg(ui::hi()),
                            ),
                            Span::styled(spark(&rx_hist, sw, net_max), Style::new().fg(ui::fg())),
                        ]),
                        Line::from(vec![
                            Span::styled(
                                crate::tr!(
                                    "up   {:>10}/s ",
                                    "yükle {:>10}/s ",
                                    ui::human(tx_s as u64)
                                ),
                                Style::new().fg(ui::hi()),
                            ),
                            Span::styled(spark(&tx_hist, sw, net_max), Style::new().fg(ui::fg())),
                        ]),
                    ])
                    .block(ui::block(ui::t("network", "ağ"))),
                    net_r,
                );

                // Disks
                let dw = mid.width.saturating_sub(4);
                let dl: Vec<Line> = disks
                    .iter()
                    .map(|d| {
                        let (t, a) = (d.total_space(), d.available_space());
                        let used = t.saturating_sub(a);
                        let mp = d.mount_point().to_string_lossy();
                        meter(
                            &format!("{:<6}", ui::truncate(&mp, 6)),
                            pct(used, t),
                            crate::tr!("{} free of {}", "{} boş / {}", ui::human(a), ui::human(t)),
                            dw,
                        )
                    })
                    .collect();
                f.render_widget(
                    Paragraph::new(dl).block(ui::block(ui::t("disks", "diskler"))),
                    mid,
                );

                // Processes
                let mut ps: Vec<_> = sys.processes().values().collect();
                ps.sort_by(|a, b| {
                    b.cpu_usage()
                        .total_cmp(&a.cpu_usage())
                        .then(b.memory().cmp(&a.memory()))
                });
                let rows = procs.height.saturating_sub(3) as usize;
                let ncpu = cpus.len().max(1) as f32;
                let mut pl = vec![Line::from(Span::styled(
                    format!("{:>7}  {:>6}  {:>10}  {}", "PID", "CPU%", "MEM", "NAME"),
                    Style::new().fg(ui::dim()),
                ))];
                pl.extend(ps.iter().take(rows).map(|p| {
                    let c = p.cpu_usage() / ncpu;
                    Line::from(vec![
                        Span::styled(
                            format!("{:>7}  ", p.pid().as_u32()),
                            Style::new().fg(ui::dim()),
                        ),
                        Span::styled(
                            format!("{c:>6.1}  "),
                            Style::new().fg(level_color(c as f64)),
                        ),
                        Span::styled(
                            format!("{:>10}  ", ui::human(p.memory())),
                            Style::new().fg(ui::hi()),
                        ),
                        Span::styled(
                            p.name().to_string_lossy().into_owned(),
                            Style::new().fg(ui::fg()),
                        ),
                    ])
                }));
                f.render_widget(
                    Paragraph::new(pl).block(ui::block(ui::t(
                        "top processes",
                        "en çok kaynak kullananlar",
                    ))),
                    procs,
                );

                f.render_widget(
                    Paragraph::new(Span::styled(
                        ui::t(" [q] quit   refresh 1s", " [q] çık   yenileme 1 sn"),
                        Style::new().fg(ui::dim()),
                    )),
                    Rect { ..foot },
                );
            })?;

            let deadline = Instant::now() + Duration::from_secs(1);
            while let Some(left) = deadline.checked_duration_since(Instant::now()) {
                if event::poll(left)? {
                    if let Some(KeyCode::Char('q') | KeyCode::Esc) = ui::pressed(&event::read()?) {
                        return Ok(());
                    }
                } else {
                    break;
                }
            }
        }
    })();
    ratatui::restore();
    res
}
