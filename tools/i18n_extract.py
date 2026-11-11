#!/usr/bin/env python3
"""Collect every English/Turkish text pair from the source into locales/source.json.

Translators work from that file: each locale/<code>.json maps the English text to
the translation. Run after changing UI text:  python tools/i18n_extract.py
Then `python tools/i18n_extract.py --missing de` lists what de.json lacks.
"""

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "src"


def lex(s):
    """Rust tokens: ('str', value) | ('id', name) | ('p', char)."""
    i, n, out = 0, len(s), []
    while i < n:
        c = s[i]
        if c.isspace():
            i += 1
        elif s.startswith("//", i):
            i = s.find("\n", i)
            i = n if i < 0 else i
        elif s.startswith("/*", i):
            i = s.find("*/", i) + 2
        elif c == "r" and re.match(r'r#*"', s[i:]):
            m = re.match(r'r(#*)"', s[i:])
            end = '"' + m.group(1)
            j = s.find(end, i + len(m.group(0)))
            out.append(("str", s[i + len(m.group(0)) : j]))
            i = j + len(end)
        elif c == "b" and s.startswith('b"', i):
            j = i + 2
            while s[j] != '"':
                j += 2 if s[j] == "\\" else 1
            i = j + 1
        elif c == '"':
            j, buf = i + 1, []
            while s[j] != '"':
                if s[j] == "\\":
                    e = s[j + 1]
                    if e == "\n":
                        j += 2
                        while s[j] in " \t\n\r":
                            j += 1
                        continue
                    if e == "u":
                        k = s.index("}", j)
                        buf.append(chr(int(s[j + 3 : k], 16)))
                        j = k + 1
                        continue
                    if e == "x":
                        buf.append(chr(int(s[j + 2 : j + 4], 16)))
                        j += 4
                        continue
                    buf.append({"n": "\n", "r": "\r", "t": "\t", "0": "\0"}.get(e, e))
                    j += 2
                else:
                    buf.append(s[j])
                    j += 1
            out.append(("str", "".join(buf)))
            i = j + 1
        elif c == "'":
            # char literal or lifetime
            m = re.match(r"'(\\.|[^\\'])'", s[i:]) or re.match(r"'\\u\{[0-9a-fA-F]+\}'", s[i:])
            if m:
                i += len(m.group(0))
            else:
                i += 1
                while i < n and (s[i].isalnum() or s[i] == "_"):
                    i += 1
        elif c.isalpha() or c == "_":
            j = i
            while j < n and (s[j].isalnum() or s[j] == "_"):
                j += 1
            out.append(("id", s[i:j]))
            i = j
        else:
            out.append(("p", c))
            i += 1
    return out


def call_args(toks, k):
    """toks[k] is '('; return the top-level argument token lists."""
    depth, args, cur = 0, [], []
    for t in toks[k:]:
        if t[0] == "p" and t[1] in "([{":
            depth += 1
            if depth == 1:
                continue
        if t[0] == "p" and t[1] in ")]}":
            depth -= 1
            if depth == 0:
                args.append(cur)
                return args
        if depth == 1 and t == ("p", ","):
            args.append(cur)
            cur = []
        else:
            cur.append(t)
    return args


def lit(arg):
    return arg[0][1] if len(arg) == 1 and arg[0][0] == "str" else None


def pairs(path, text):
    toks = lex(text)
    for k, t in enumerate(toks):
        nxt = toks[k + 1] if k + 1 < len(toks) else None
        prev = toks[k - 1] if k else None
        if t == ("id", "tr") and nxt == ("p", "!") and toks[k + 2] == ("p", "("):
            a = call_args(toks, k + 2)
            if len(a) >= 2 and lit(a[0]) is not None and lit(a[1]) is not None:
                yield lit(a[0]), lit(a[1]), "fmt"
        elif t == ("id", "t") and nxt == ("p", "(") and prev not in (("p", "."), ("id", "fn")):
            a = call_args(toks, k + 1)
            if len(a) == 2 and lit(a[0]) is not None and lit(a[1]) is not None:
                yield lit(a[0]), lit(a[1]), "plain"
        elif path.name == "catalog.rs" and t[0] == "id" and t[1] in ("r", "ra") and nxt == ("p", "(") and prev != ("id", "fn"):
            a = [lit(x) for x in call_args(toks, k + 1)]
            s = [x for x in a if x is not None]
            if len(s) >= 3:  # group, english, turkish
                yield s[1], s[2], "plain"
        elif t[0] == "id" and t[1] in ("en", "detail_en") and nxt == ("p", ":") and toks[k + 2][0] == "str":
            want = "tr" if t[1] == "en" else "detail_tr"
            for j in range(k + 3, min(k + 12, len(toks) - 2)):
                if toks[j] == ("id", want) and toks[j + 1] == ("p", ":") and toks[j + 2][0] == "str":
                    yield toks[k + 2][1], toks[j + 2][1], "plain"
                    break
    # tuple tables: (const name, index of English among the tuple's top-level strings)
    for name, idx in (("MENU", 1), ("PLANS", 2), ("CATEGORIES", 1)):
        for k, t in enumerate(toks):
            if t == ("id", name) and toks[k + 1] == ("p", ":") and toks[k - 1] == ("id", "const"):
                j = k
                while toks[j] != ("p", "="):
                    j += 1
                j += 1  # '&'
                while toks[j] != ("p", "["):
                    j += 1
                for row in call_args(toks, j):
                    if not row or row[0] != ("p", "("):
                        continue
                    strs = [lit(x) for x in call_args(row, 0)]
                    strs = [x for x in strs if x is not None]
                    if len(strs) > idx + 1 and strs[idx]:
                        yield strs[idx], strs[idx + 1], "plain"
    # PowerShell: T 'english' 'turkish'
    for m in re.finditer(r"(?<![\w$-])T '((?:[^']|'')*)' '((?:[^']|'')*)'", text):
        yield m.group(1).replace("''", "'"), m.group(2).replace("''", "'"), "ps"


def collect():
    seen, out = set(), []
    for path in sorted(SRC.glob("*.rs")):
        if path.name == "i18n.rs":
            continue
        for en, tr, kind in pairs(path, path.read_text(encoding="utf-8")):
            if en and en not in seen:
                seen.add(en)
                out.append({"en": en, "tr": tr, "kind": kind, "file": path.name})
    return out


def main():
    src = collect()
    if len(sys.argv) == 3 and sys.argv[1] == "--missing":
        have = json.loads((ROOT / "locales" / f"{sys.argv[2]}.json").read_text(encoding="utf-8"))
        miss = [s for s in src if not have.get(s["en"])]
        print(json.dumps(miss, ensure_ascii=False, indent=1))
        print(f"{len(miss)} missing of {len(src)}", file=sys.stderr)
        return
    (ROOT / "locales" / "source.json").write_text(
        json.dumps(src, ensure_ascii=False, indent=1) + "\n", encoding="utf-8", newline="\n"
    )
    print(f"{len(src)} strings -> locales/source.json")


main()
