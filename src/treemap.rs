//! ASCII treemap: every item is a box whose area matches its size.
//! Layout is "squarified" (Bruls, Huizing, van Wijk) so boxes stay readable.

use crate::ui;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct R {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

fn worst(row: &[f64], side: f64) -> f64 {
    let s: f64 = row.iter().sum();
    let (mx, mn) = row
        .iter()
        .fold((f64::MIN, f64::MAX), |(a, b), &v| (a.max(v), b.min(v)));
    let s2 = s * s;
    let w2 = side * side;
    (w2 * mx / s2).max(s2 / (w2 * mn))
}

/// Lay `values` (any scale, sorted descending works best) into `area`.
pub fn squarify(values: &[f64], area: R) -> Vec<R> {
    let total: f64 = values.iter().sum();
    if total <= 0.0 || values.is_empty() {
        return vec![];
    }
    let scale = area.w * area.h / total;
    let v: Vec<f64> = values.iter().map(|x| (x * scale).max(1e-9)).collect();
    let mut out = Vec::with_capacity(v.len());
    let mut rest = area;
    let mut i = 0;
    while i < v.len() {
        let side = rest.w.min(rest.h);
        let mut row = vec![v[i]];
        let mut j = i + 1;
        while j < v.len() {
            let mut next = row.clone();
            next.push(v[j]);
            if worst(&next, side) > worst(&row, side) {
                break;
            }
            row = next;
            j += 1;
        }
        let sum: f64 = row.iter().sum();
        if rest.w >= rest.h {
            // column on the left
            let cw = sum / rest.h;
            let mut y = rest.y;
            for a in &row {
                let h = a / cw;
                out.push(R {
                    x: rest.x,
                    y,
                    w: cw,
                    h,
                });
                y += h;
            }
            rest = R {
                x: rest.x + cw,
                w: (rest.w - cw).max(0.0),
                ..rest
            };
        } else {
            // row on top
            let rh = sum / rest.w;
            let mut x = rest.x;
            for a in &row {
                let w = a / rh;
                out.push(R {
                    x,
                    y: rest.y,
                    w,
                    h: rh,
                });
                x += w;
            }
            rest = R {
                y: rest.y + rh,
                h: (rest.h - rh).max(0.0),
                ..rest
            };
        }
        i = j;
    }
    out
}

/// Draw `items` (label, size) into a `w` x `h` character grid. `selected`
/// is highlighted. Boxes too small for a label are shaded with '.'.
pub fn render(
    items: &[(String, u64)],
    selected: Option<usize>,
    w: u16,
    h: u16,
) -> Vec<Line<'static>> {
    let (w, h) = (w as usize, h as usize);
    if w < 4 || h < 2 {
        return vec![];
    }
    let values: Vec<f64> = items.iter().map(|i| i.1 as f64).collect();
    let rects = squarify(
        &values,
        R {
            x: 0.0,
            y: 0.0,
            w: w as f64,
            h: h as f64,
        },
    );
    // cell = (char, owner index)
    let mut grid: Vec<Vec<(char, usize)>> = vec![vec![(' ', usize::MAX); w]; h];
    for (idx, r) in rects.iter().enumerate() {
        let x0 = r.x.round() as usize;
        let y0 = r.y.round() as usize;
        let x1 = ((r.x + r.w).round() as usize).min(w);
        let y1 = ((r.y + r.h).round() as usize).min(h);
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        let (bw, bh) = (x1 - x0, y1 - y0);
        for (y, row) in grid.iter_mut().enumerate().take(y1).skip(y0) {
            for (x, cell) in row.iter_mut().enumerate().take(x1).skip(x0) {
                let edge_y = y == y0 || y == y1 - 1;
                let edge_x = x == x0 || x == x1 - 1;
                let c = if bw < 3 || bh < 2 {
                    '.'
                } else if edge_y && edge_x {
                    '+'
                } else if edge_y {
                    '-'
                } else if edge_x {
                    '|'
                } else {
                    ' '
                };
                *cell = (c, idx);
            }
        }
        // label inside the box
        if bw >= 5 && bh >= 3 {
            let inner = bw - 2;
            let mut put = |line: usize, text: &str| {
                if y0 + line < y1 - 1 {
                    // a CJK character takes two cells; the second one is a '\0' placeholder
                    let mut col = 0;
                    for ch in text.chars() {
                        let w = ui::width(ch.encode_utf8(&mut [0; 4])).max(1);
                        if col + w > inner {
                            break;
                        }
                        grid[y0 + line][x0 + 1 + col] = (ch, idx);
                        if w == 2 {
                            grid[y0 + line][x0 + 2 + col] = ('\0', idx);
                        }
                        col += w;
                    }
                }
            };
            put(1, &items[idx].0);
            put(2, &ui::human(items[idx].1));
        }
    }
    let palette = [ui::fg(), ui::hi(), Color::Cyan, Color::Magenta];
    grid.into_iter()
        .map(|row| {
            let mut spans: Vec<Span<'static>> = vec![];
            let mut cur = String::new();
            let mut cur_owner = usize::MAX;
            let flush = |spans: &mut Vec<Span<'static>>, text: &mut String, owner: usize| {
                if text.is_empty() {
                    return;
                }
                let mut st = Style::new().fg(if owner == usize::MAX {
                    ui::dim()
                } else {
                    palette[owner % palette.len()]
                });
                if Some(owner) == selected {
                    st = st.add_modifier(Modifier::REVERSED | Modifier::BOLD);
                }
                spans.push(Span::styled(std::mem::take(text), st));
            };
            for (c, owner) in row {
                if owner != cur_owner {
                    flush(&mut spans, &mut cur, cur_owner);
                    cur_owner = owner;
                }
                if c != '\0' {
                    cur.push(c);
                }
            }
            flush(&mut spans, &mut cur, cur_owner);
            Line::from(spans)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wide (CJK) names must not push the box edges out of line.
    #[test]
    fn wide_names_keep_rows_the_same_width() {
        let items = vec![
            ("写真フォルダー".to_string(), 600),
            ("ドキュメント".to_string(), 300),
            ("x".to_string(), 100),
        ];
        let lines = render(&items, None, 40, 10);
        for l in &lines {
            let text: String = l.spans.iter().map(|s| s.content.as_ref()).collect();
            assert_eq!(ui::width(&text), 40, "{text}");
        }
    }

    #[test]
    fn layout_covers_area_proportionally() {
        let area = R {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 40.0,
        };
        let vals = [50.0, 25.0, 12.5, 6.25, 6.25];
        let rs = squarify(&vals, area);
        assert_eq!(rs.len(), vals.len());
        let total: f64 = rs.iter().map(|r| r.w * r.h).sum();
        assert!((total - 4000.0).abs() < 1e-6);
        for (r, v) in rs.iter().zip(vals) {
            assert!((r.w * r.h - v / 100.0 * 4000.0).abs() < 1e-6);
            assert!(
                r.x >= -1e-9
                    && r.y >= -1e-9
                    && r.x + r.w <= 100.0 + 1e-6
                    && r.y + r.h <= 40.0 + 1e-6
            );
        }
    }

    #[test]
    fn renders_labels() {
        let items = vec![
            ("Videos".to_string(), 600),
            ("Music".to_string(), 300),
            ("x".to_string(), 100),
        ];
        let lines = render(&items, Some(0), 40, 10);
        assert_eq!(lines.len(), 10);
        let text: String = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect::<String>()
                    + "\n"
            })
            .collect();
        assert!(text.contains("Videos") && text.contains("Music"));
        assert!(text.is_ascii(), "treemap must stay ASCII");
    }
}
