#!/usr/bin/env python3
"""Check locales/<code>.json against locales/source.json.

Usage: python tools/i18n_check.py [code ...]   (default: every locale)
Fails on invalid JSON, placeholders that differ from the English text,
unbalanced braces, missing entries and keys whose English text no longer exists.
"""

import json
import pathlib
import re
import sys
from collections import Counter

ROOT = pathlib.Path(__file__).resolve().parent.parent
LOC = ROOT / "locales"


def holes(s):
    out, i = [], 0
    while i < len(s):
        if s.startswith("{{", i) or s.startswith("}}", i):
            i += 2
        elif s[i] == "{":
            j = s.find("}", i)
            if j < 0:
                return None
            out.append(s[i + 1 : j].split(":")[0].strip())
            i = j + 1
        elif s[i] == "}":
            return None
        else:
            i += 1
    return Counter(out)


def check(code):
    src = json.loads((LOC / "source.json").read_text(encoding="utf-8"))
    try:
        tr = json.loads((LOC / f"{code}.json").read_text(encoding="utf-8"))
    except Exception as e:  # noqa: BLE001
        print(f"{code}: invalid JSON: {e}")
        return False
    ok, missing = True, 0
    keys = {s["en"] for s in src}
    for s in src:
        t = tr.get(s["en"])
        if not t:
            missing += 1
            continue
        if s["kind"] != "plain" and holes(t) != holes(s["en"]):
            ok = False
            print(f"{code}: placeholders differ\n   en: {s['en']!r}\n   {code}: {t!r}")
        if s["en"].startswith(" ") != t.startswith(" ") or s["en"].endswith("\n") != t.endswith("\n"):
            print(f"{code}: (warning) leading space / trailing newline differs: {s['en']!r}")
        if s["kind"] == "ps" and "'" in t:
            pass  # passed to PowerShell as JSON, quotes are fine
    extra = [k for k in tr if k not in keys]
    print(f"{code}: {len(src) - missing}/{len(src)} translated, {missing} missing, {len(extra)} unknown keys")
    # every language must be complete: a missing entry shows English, an unknown
    # key usually means the English text changed and the translation was left behind
    for k in extra:
        print(f"{code}: unknown key (English text changed?): {k!r}")
    return ok and missing == 0 and not extra


codes = sys.argv[1:] or sorted(p.stem for p in LOC.glob("*.json") if p.stem != "source")
sys.exit(0 if all([check(c) for c in codes]) else 1)
