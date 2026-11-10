"""Render `tmux capture-pane -e -p` output (ANSI colors) to a PNG.

Used for the README screenshots:
    tmux capture-pane -e -p -t SESSION > shot.ansi
    python3 docs/screenshots/ansi2png.py shot.ansi shot.png "kz"
"""
import re
import sys

from PIL import Image, ImageDraw, ImageFont

FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
FONT_B = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf"
SCALE = 2
SIZE = 15 * SCALE
BG = (12, 12, 12)
FG = (204, 204, 204)

BASE16 = [
    (12, 12, 12), (197, 15, 31), (19, 161, 14), (193, 156, 0),
    (0, 55, 218), (136, 23, 152), (58, 150, 221), (204, 204, 204),
    (118, 118, 118), (231, 72, 86), (22, 198, 12), (249, 241, 165),
    (59, 120, 255), (180, 0, 158), (97, 214, 214), (242, 242, 242),
]  # Windows Terminal "Campbell" palette


def xterm256(n):
    if n < 16:
        return BASE16[n]
    if n < 232:
        n -= 16
        steps = [0, 95, 135, 175, 215, 255]
        return (steps[n // 36], steps[(n // 6) % 6], steps[n % 6])
    v = 8 + (n - 232) * 10
    return (v, v, v)


def parse(text):
    """Yield rows of (char, fg, bg, bold)."""
    rows = []
    fg, bg, bold, rev = FG, None, False, False
    for line in text.split("\n"):
        row = []
        for tok in re.split(r"(\x1b\[[0-9;]*m)", line):
            m = re.fullmatch(r"\x1b\[([0-9;]*)m", tok)
            if not m:
                for ch in tok:
                    f, b = (bg or BG, fg) if rev else (fg, bg)
                    row.append((ch, f, b, bold))
                continue
            codes = [int(c) if c else 0 for c in m.group(1).split(";")] or [0]
            i = 0
            while i < len(codes):
                c = codes[i]
                if c == 0:
                    fg, bg, bold, rev = FG, None, False, False
                elif c == 1:
                    bold = True
                elif c == 22:
                    bold = False
                elif c == 7:
                    rev = True
                elif c == 27:
                    rev = False
                elif 30 <= c <= 37:
                    fg = BASE16[c - 30]
                elif 90 <= c <= 97:
                    fg = BASE16[c - 90 + 8]
                elif 40 <= c <= 47:
                    bg = BASE16[c - 40]
                elif 100 <= c <= 107:
                    bg = BASE16[c - 100 + 8]
                elif c == 39:
                    fg = FG
                elif c == 49:
                    bg = None
                elif c in (38, 48) and i + 2 < len(codes) and codes[i + 1] == 5:
                    col = xterm256(codes[i + 2])
                    if c == 38:
                        fg = col
                    else:
                        bg = col
                    i += 2
                elif c in (38, 48) and i + 4 < len(codes) and codes[i + 1] == 2:
                    col = tuple(codes[i + 2:i + 5])
                    if c == 38:
                        fg = col
                    else:
                        bg = col
                    i += 4
                i += 1
        rows.append(row)
    while rows and not any(ch.strip() for ch, *_ in rows[-1]):
        rows.pop()
    return rows


def render(rows, out, title):
    font = ImageFont.truetype(FONT, SIZE)
    bold = ImageFont.truetype(FONT_B, SIZE)
    cw = font.getbbox("M")[2]
    ch = int(SIZE * 1.22)
    cols = max(len(r) for r in rows)
    pad = 18 * SCALE
    bar = 34 * SCALE if title else 0
    w, h = cols * cw + 2 * pad, len(rows) * ch + 2 * pad + bar
    img = Image.new("RGB", (w, h), BG)
    d = ImageDraw.Draw(img)
    if title:
        d.rectangle([0, 0, w, bar], fill=(32, 32, 32))
        tw = font.getbbox(title)[2]
        d.text(((w - tw) // 2, bar // 2 - SIZE // 2 - 2), title, font=font, fill=(160, 160, 160))
    for y, row in enumerate(rows):
        for x, (c, f, b, bo) in enumerate(row):
            px, py = pad + x * cw, bar + pad + y * ch
            if b is not None:
                d.rectangle([px, py, px + cw, py + ch], fill=b)
            if c != " ":
                d.text((px, py + (ch - SIZE) // 2), c, font=bold if bo else font, fill=f)
    img.save(out, optimize=True)


if __name__ == "__main__":
    text = open(sys.argv[1], encoding="utf-8").read()
    render(parse(text), sys.argv[2], sys.argv[3] if len(sys.argv) > 3 else "")
