#!/usr/bin/env python3
"""Find interface layout problems in kazma output, in any language.

Usage:
  python tools/ui_check.py LANG FILE [FILE ...]      check captured output files
  python tools/ui_check.py --screen LANG WIDTH FILE  check a full-screen capture (tmux capture-pane -p)

Checks:
  box      every line of an ASCII box is as wide as its top edge and ends with '|'
  column   lines of one block put their second column at the same display column
  wide     no line is wider than the terminal (screen mode) or 120 columns
  english  no English UI text is left where the language has a translation

Display width counts CJK characters as two columns, like a terminal does.
Exit code 1 if anything is found.
"""

import json
import pathlib
import re
import sys
import unicodedata

ROOT = pathlib.Path(__file__).resolve().parent.parent
ANSI = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]")


def width(s):
    w = 0
    for c in s:
        if unicodedata.combining(c):
            continue
        w += 2 if unicodedata.east_asian_width(c) in "WF" else 1
    return w


def clean(text):
    text = ANSI.sub("", text).replace("\r\n", "\n")
    # a progress line overwritten with \r: keep what is left visible
    return [line.split("\r")[-1].rstrip() for line in text.split("\n")]


TOP = re.compile(r"^(\s*)\.(\[ .* \])?-+\.$")
BOTTOM = re.compile(r"^(\s*)'-+'$")


def check_boxes(lines, where):
    out = []
    i = 0
    while i < len(lines):
        m = TOP.match(lines[i])
        if not m:
            i += 1
            continue
        indent, w = len(m.group(1)), width(lines[i])
        j = i + 1
        while j < len(lines) and not BOTTOM.match(lines[j]) and j - i < 400:
            line = lines[j]
            if TOP.match(line):
                break  # nested/adjacent boxes (side by side panels) are not checked
            if line.strip() and line[:indent].strip() == "" and line[indent:indent + 1] == "|":
                if width(line) != w or not line.endswith("|"):
                    out.append(f"{where}:{j + 1}: box line is {width(line)} wide, box is {w}: {line.strip()[:90]}")
            j += 1
        if j < len(lines) and BOTTOM.match(lines[j]) and width(lines[j]) != w:
            out.append(f"{where}:{j + 1}: box bottom is {width(lines[j])} wide, top is {w}")
        i = j + 1
    return out


GAP = re.compile(r"\S(\s{2,})\S")


def second_field(line):
    """Display columns where the text after the first wide gap starts and ends
    (it ends at the next wide gap or the end of the line)."""
    m = GAP.search(line)
    start = width(line[: m.end() - 1])
    rest = line[m.end() - 1:]
    n = re.search(r"\s{2,}", rest)
    field = rest[: n.start()] if n else rest
    return start, start + width(field)


def check_columns(lines, where):
    """Inside a block of similar lines, the second column must line up: either
    its start (left-aligned text) or its end (right-aligned numbers) matches the
    rest of the block. Flags lines that are off by 1-6 columns."""
    out = []
    block = []

    def mode(values):
        return max(set(values), key=values.count)

    def flush():
        # compare only lines of the same shape: second column is a number, a bar or text
        def shape(line):
            m = GAP.search(line)
            c = line[m.end() - 1]
            return "digit" if c.isdigit() else "bar" if c == "[" else "text"

        shapes = {}
        for n, line in block:
            shapes.setdefault(shape(line), []).append((n, line))
        groups = [g for g in shapes.values() if len(g) >= 3]
        block.clear()
        for g in groups:
            block.extend(g)
            check_block()
            block.clear()

    def check_block():
        if len(block) >= 3:
            fields = [(n, line, second_field(line)) for n, line in block]
            starts = [f[0] for _, _, f in fields]
            ends = [f[1] for _, _, f in fields]
            ms, me = mode(starts), mode(ends)
            for n, line, (st, en) in fields:
                if st == ms or en == me:
                    continue
                off = min(abs(st - ms), abs(en - me))
                if 0 < off <= 6:
                    out.append(f"{where}:{n + 1}: column starts at {st}/ends at {en}, block uses {ms}/{me}: {line.strip()[:90]}")
        block.clear()

    prefix = None
    for n, line in enumerate(lines):
        m = GAP.search(line)
        # a block: same indentation and same kind of marker ([ok], [..], digits ...)
        key = None
        if m and line.strip() and not TOP.match(line) and not BOTTOM.match(line):
            lead = re.match(r"^(\s*)(\[..\]|[|>]|)", line)
            key = (len(lead.group(1)), lead.group(2)[:1])
        if key is None or key != prefix:
            flush()
            prefix = key
        if key is not None:
            block.append((n, line))
    flush()
    return out


def english_keys(lang):
    if lang in ("en",):
        return []
    src = json.loads((ROOT / "locales" / "source.json").read_text(encoding="utf-8"))
    table = {}
    if lang != "tr":
        p = ROOT / "locales" / f"{lang}.json"
        if p.exists():
            table = json.loads(p.read_text(encoding="utf-8"))
    keys = []
    for s in src:
        en = s["en"]
        tr = s["tr"] if lang == "tr" else table.get(en, "")
        if not tr or tr == en:
            continue
        # the longest literal piece of the English text, ignoring placeholders
        pieces = re.split(r"\{[^{}]*\}", en) if s["kind"] != "plain" else [en]
        piece = max((p.strip() for p in pieces), key=len, default="")
        if len(piece) >= 14 and " " in piece and piece not in tr:
            keys.append(piece)
    return keys


def check_english(lines, where, keys):
    out = []
    text = "\n".join(lines)
    for k in keys:
        i = text.find(k)
        if i >= 0:
            n = text.count("\n", 0, i)
            out.append(f"{where}:{n + 1}: untranslated: {k!r}")
    return out


def check_wide(lines, where, limit):
    return [f"{where}:{n + 1}: {width(l)} columns wide (limit {limit}): {l.strip()[:60]}"
            for n, l in enumerate(lines) if width(l) > limit]


def main():
    args = sys.argv[1:]
    problems = []
    screen = bool(args) and args[0] == "--screen"
    if screen:
        lang, w, files = args[1], int(args[2]), args[3:]
        limit = w
    else:
        lang, files = args[0], args[1:]
        limit = 120
    keys = english_keys(lang)
    for f in files:
        lines = clean(pathlib.Path(f).read_text(encoding="utf-8", errors="replace"))
        name = pathlib.Path(f).name
        problems += check_boxes(lines, name)
        if not screen:
            problems += check_columns(lines, name)
        problems += check_wide(lines, name, limit)
        problems += check_english(lines, name, keys)
    for p in problems:
        print(p)
    print(f"{lang}: {len(files)} file(s), {len(problems)} problem(s)", file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
