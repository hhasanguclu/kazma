//! `kz status`: live system dashboard.

use crate::{oplog, ui};
use crossterm::event::{self, KeyCode};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
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
        .saturating_sub(ui::width(label) + ui::width(&extra) + 12)
        .clamp(5, 60);
    Line::from(
        vec![Span::styled(
            format!("{label} "),
            Style::new().fg(ui::fg()).add_modifier(Modifier::BOLD),
        )]
        .into_iter()
        .chain(ui::bar_spans(pct / 100.0, bar_w, level_color(pct)))
        .chain([
            Span::styled(format!(" {pct:>5.1}% "), Style::new().fg(ui::hi())),
            Span::styled(extra, Style::new().fg(ui::dim())),
        ])
        .collect::<Vec<_>>(),
    )
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

    let mut terminal = ui::tui_start();
    // process panel state
    let mut pstate = ListState::default();
    let mut sel_pid: Option<u32> = None;
    let mut search = ui::Search::default();
    let mut by_mem = false;
    let mut confirm: Option<(u32, String)> = None;
    let mut msg = String::new();
    let mut proc_area = Rect::default();
    let (mut rx_s, mut tx_s) = (0.0f64, 0.0f64);
    let mut first = true;
    // GPU via performance counters, battery, and the "why is it running" panel
    let mut gpu = crate::gpu::Gpu::new();
    let (mut gpu_pct, mut gpu_mem): (Option<f64>, Option<u64>) = (None, None);
    let mut detail: Option<Vec<(String, String)>> = None;
    let mut startup_cache: Option<Vec<crate::startup::Entry>> = None;
    let res = (|| -> anyhow::Result<()> {
        loop {
            if first || last.elapsed() >= Duration::from_secs(1) {
                first = false;
                let dt = last.elapsed().as_secs_f64().max(0.001);
                last = Instant::now();
                sys.refresh_cpu_usage();
                sys.refresh_memory();
                sys.refresh_processes(ProcessesToUpdate::All, true);
                nets.refresh(true);
                if let Some(g) = gpu.as_mut() {
                    (gpu_pct, gpu_mem) = g.sample();
                }
                if tick.is_multiple_of(5) {
                    disks.refresh(true);
                }
                tick += 1;

                let (rx, tx) = nets.iter().fold((0u64, 0u64), |(r, t), (_, d)| {
                    (r + d.received(), t + d.transmitted())
                });
                (rx_s, tx_s) = (rx as f64 / dt, tx as f64 / dt);
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
            }

            // processes: filter, sort, keep the selection on the same pid
            let ncpu = sys.cpus().len().max(1) as f32;
            let mut plist: Vec<(u32, String, f32, u64)> = sys
                .processes()
                .values()
                .map(|p| {
                    (
                        p.pid().as_u32(),
                        p.name().to_string_lossy().into_owned(),
                        p.cpu_usage() / ncpu,
                        p.memory(),
                    )
                })
                .filter(|p| search.matches(&p.1))
                .collect();
            if by_mem {
                plist.sort_by(|a, b| b.3.cmp(&a.3).then(b.2.total_cmp(&a.2)));
            } else {
                plist.sort_by(|a, b| b.2.total_cmp(&a.2).then(b.3.cmp(&a.3)));
            }
            let sel = sel_pid
                .and_then(|pid| plist.iter().position(|p| p.0 == pid))
                .unwrap_or(
                    pstate
                        .selected()
                        .unwrap_or(0)
                        .min(plist.len().saturating_sub(1)),
                );
            pstate.select((!plist.is_empty()).then_some(sel));
            sel_pid = plist.get(sel).map(|p| p.0);

            terminal.draw(|f| {
                let cpus = sys.cpus();
                let core_rows = cpus.len().div_ceil(2) as u16;
                let [head, cpu, mem, mid, procs, foot] = Layout::vertical([
                    Constraint::Length(3),
                    Constraint::Length(core_rows + 4),
                    Constraint::Length(if gpu_pct.is_some() { 5 } else { 4 }),
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
                    Span::styled(
                        match crate::gpu::battery() {
                            Some((pct, true)) => {
                                crate::tr!("   battery {pct}% (charging)", "   pil %{pct} (şarjda)")
                            }
                            Some((pct, false)) => crate::tr!("   battery {pct}%", "   pil %{pct}"),
                            None => String::new(),
                        },
                        Style::new().fg(ui::hi()),
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
                    Paragraph::new(
                        vec![
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
                        ]
                        .into_iter()
                        .chain(gpu_pct.map(|g| {
                            meter(
                                "GPU ",
                                g,
                                gpu_mem
                                    .map(|m| format!("VRAM {}", ui::human(m)))
                                    .unwrap_or_default(),
                                w,
                            )
                        }))
                        .collect::<Vec<_>>(),
                    )
                    .block(ui::block(ui::t("memory", "bellek"))),
                    mem_l,
                );
                let nw = net_r.width.saturating_sub(4) as usize;
                let net_max = rx_hist
                    .iter()
                    .chain(tx_hist.iter())
                    .cloned()
                    .fold(1024.0, f64::max);
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
                proc_area = procs;
                let head = format!(
                    "  {:>7}  {:>6}  {:>10}  {}",
                    "PID",
                    "CPU%",
                    ui::t("MEM", "BELLEK"),
                    ui::t("NAME", "AD")
                );
                let rows: Vec<ListItem> = std::iter::once(ListItem::new(Line::from(Span::styled(
                    head,
                    Style::new().fg(ui::dim()),
                ))))
                .chain(plist.iter().map(|(pid, name, c, mem)| {
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("{pid:>7}  "), Style::new().fg(ui::dim())),
                        Span::styled(
                            format!("{c:>6.1}  "),
                            Style::new().fg(level_color(*c as f64)),
                        ),
                        Span::styled(
                            format!("{:>10}  ", ui::human(*mem)),
                            Style::new().fg(ui::hi()),
                        ),
                        Span::styled(name.clone(), Style::new().fg(ui::fg())),
                    ]))
                }))
                .collect();
                let title = format!(
                    "{} ({}, {})",
                    ui::t("processes", "süreçler"),
                    plist.len(),
                    if by_mem {
                        ui::t("by memory", "belleğe göre")
                    } else {
                        ui::t("by cpu", "cpu'ya göre")
                    }
                );
                // row 0 is the header, so the list state is shifted by one
                let mut shown =
                    ListState::default().with_selected(pstate.selected().map(|i| i + 1));
                *shown.offset_mut() = pstate.offset();
                f.render_stateful_widget(
                    List::new(rows)
                        .block(ui::block(&title))
                        .highlight_style(ui::sel_style())
                        .highlight_symbol(">"),
                    procs,
                    &mut shown,
                );
                *pstate.offset_mut() = shown.offset();

                let footer = if let Some((pid, name)) = &confirm {
                    Line::from(Span::styled(
                        crate::tr!(
                            " end {name} (pid {pid})? [y/N]",
                            " {name} (pid {pid}) sonlandırılsın mı? [y/N]"
                        ),
                        Style::new().fg(ui::warn_c()).bold(),
                    ))
                } else if let Some(st) = search.status() {
                    Line::from(Span::styled(st, Style::new().fg(ui::hi()).bold()))
                } else if !msg.is_empty() {
                    Line::from(Span::styled(format!(" {msg}"), Style::new().fg(ui::hi())))
                } else {
                    ui::keys(&[
                        (ui::t("up/down", "yukarı/aşağı"), ui::t("select", "seç")),
                        ("enter", ui::t("why is it running?", "neden çalışıyor?")),
                        ("x", ui::t("end process", "sonlandır")),
                        ("m", ui::t("cpu / memory", "cpu / bellek")),
                        ("/", ui::t("search", "ara")),
                        ("q", ui::t("quit", "çık")),
                    ])
                };
                f.render_widget(Paragraph::new(footer), Rect { ..foot });

                // "why is this running" panel over the process list
                if let Some(lines) = &detail {
                    let area = f.area();
                    let w = area.width.saturating_sub(8).min(100);
                    // long values (paths, command lines) wrap under the value column
                    let room = (w as usize).saturating_sub(2 + 17).max(10);
                    let why = ui::t("why", "neden");
                    let mut text: Vec<Line> = vec![];
                    for (k, v) in lines {
                        let color = if k.is_empty() || k == why {
                            ui::hi()
                        } else {
                            ui::fg()
                        };
                        for (i, part) in ui::wrap(v, room).into_iter().enumerate() {
                            let label = if i == 0 {
                                ui::pad(k, 16)
                            } else {
                                " ".repeat(16)
                            };
                            text.push(Line::from(vec![
                                Span::styled(format!(" {label}"), Style::new().fg(ui::dim())),
                                Span::styled(part, Style::new().fg(color)),
                            ]));
                        }
                    }
                    let h = (text.len() as u16 + 4).min(area.height.saturating_sub(2));
                    let r = Rect {
                        x: area.x + (area.width - w) / 2,
                        y: area.y + (area.height - h) / 2,
                        width: w,
                        height: h,
                    };
                    let text: Vec<Line> = text
                        .into_iter()
                        .chain([
                            Line::default(),
                            ui::keys(&[
                                ("enter / esc", ui::t("close", "kapat")),
                                ("x", ui::t("end process", "sonlandır")),
                            ]),
                        ])
                        .collect();
                    f.render_widget(ratatui::widgets::Clear, r);
                    f.render_widget(
                        Paragraph::new(text).block(ui::block(ui::t(
                            "why is this running?",
                            "bu neden çalışıyor?",
                        ))),
                        r,
                    );
                }
            })?;

            if !event::poll(Duration::from_millis(150))? {
                continue;
            }
            let ev = event::read()?;
            if let Some((pid, name)) = confirm.take() {
                if let Some(KeyCode::Char('y' | 'Y')) = ui::pressed(&ev) {
                    let done = sys
                        .process(sysinfo::Pid::from_u32(pid))
                        .is_some_and(|p| p.kill());
                    msg = if done {
                        oplog::record("status", &format!("ended {name} pid={pid}"));
                        crate::tr!("ended {name}", "{name} sonlandırıldı")
                    } else {
                        crate::tr!(
                            "could not end {name} (try an elevated terminal)",
                            "{name} sonlandırılamadı (yönetici terminalini deneyin)"
                        )
                    };
                }
                continue;
            }
            let n = plist.len();
            let cur = pstate.selected().unwrap_or(0);
            let mut go = |i: usize| sel_pid = plist.get(i.min(n.saturating_sub(1))).map(|p| p.0);
            // the panel's first row is the header
            let area = Rect {
                y: proc_area.y + 1,
                height: proc_area.height.saturating_sub(1),
                ..proc_area
            };
            if let Some(m) = ui::mouse(&ev, area, pstate.offset(), n) {
                match m {
                    ui::Mouse::ScrollUp => go(cur.saturating_sub(3)),
                    ui::Mouse::ScrollDown => go(cur + 3),
                    ui::Mouse::Click { index, .. } => go(index),
                }
                continue;
            }
            let Some(code) = ui::pressed(&ev) else {
                continue;
            };
            msg.clear();
            if detail.is_some() {
                match code {
                    KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => detail = None,
                    KeyCode::Char('x') => {
                        detail = None;
                        confirm = plist
                            .get(pstate.selected().unwrap_or(0))
                            .map(|p| (p.0, p.1.clone()));
                    }
                    _ => {}
                }
                continue;
            }
            if search.key(code) {
                sel_pid = None;
                pstate.select(Some(0));
                continue;
            }
            match code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Up | KeyCode::Char('k') => go(cur.saturating_sub(1)),
                KeyCode::Down | KeyCode::Char('j') => go(cur + 1),
                KeyCode::PageUp => go(cur.saturating_sub(10)),
                KeyCode::PageDown => go(cur + 10),
                KeyCode::Home => go(0),
                KeyCode::End => go(n),
                KeyCode::Enter => {
                    if let Some(p) = plist.get(cur) {
                        if startup_cache.is_none() {
                            startup_cache = Some(if cfg!(windows) {
                                crate::startup::list().unwrap_or_default()
                            } else {
                                vec![]
                            });
                        }
                        let mut lines = vec![(
                            ui::t("process", "süreç").to_string(),
                            format!("{} ({})", p.1, p.0),
                        )];
                        lines.extend(crate::procinfo::explain(
                            &sys,
                            p.0,
                            startup_cache.as_deref().unwrap_or(&[]),
                        ));
                        detail = Some(lines);
                    }
                }
                KeyCode::Char('m') => {
                    by_mem = !by_mem;
                    sel_pid = None;
                    pstate.select(Some(0));
                }
                KeyCode::Char('x') | KeyCode::Delete => {
                    confirm = plist.get(cur).map(|p| (p.0, p.1.clone()));
                }
                _ => {}
            }
        }
    })();
    ui::tui_end();
    res
}
