#!/usr/bin/env python3
"""Write the Scoop and winget manifests for a kazma release.

Usage: python packaging/update_manifests.py VERSION SHA256

- bucket/kazma.json                         Scoop (this repo is the bucket)
- packaging/winget/VERSION/*.yaml           winget, ready for `wingetcreate submit`

CI runs this after every release; it only uses the standard library.
"""

import json
import pathlib
import re
import sys

REPO = "hhasanguclu/kazma"
ID = "hhasanguclu.kazma"
DESC = "Oldschool ASCII system cleaner, disk explorer and toolkit for Windows"
ROOT = pathlib.Path(__file__).resolve().parent.parent


def scoop(version: str, sha: str) -> dict:
    url = f"https://github.com/{REPO}/releases/download/v{version}/kz.exe"
    return {
        "version": version,
        "description": DESC,
        "homepage": f"https://github.com/{REPO}",
        "license": "MIT",
        "architecture": {"64bit": {"url": url, "hash": sha}},
        "bin": "kz.exe",
        "checkver": "github",
        "autoupdate": {
            "architecture": {
                "64bit": {"url": f"https://github.com/{REPO}/releases/download/v$version/kz.exe"}
            },
            "hash": {"url": "$baseurl/SHA256SUMS.txt"},
        },
    }


def winget(version: str, sha: str) -> dict[str, str]:
    url = f"https://github.com/{REPO}/releases/download/v{version}/kz.exe"
    head = "# Created by kazma CI (packaging/update_manifests.py)\n"
    return {
        f"{ID}.yaml": head
        + f"PackageIdentifier: {ID}\nPackageVersion: {version}\nDefaultLocale: en-US\n"
        "ManifestType: version\nManifestVersion: 1.6.0\n",
        f"{ID}.locale.en-US.yaml": head
        + f"PackageIdentifier: {ID}\nPackageVersion: {version}\nPackageLocale: en-US\n"
        "Publisher: hhasanguclu\n"
        f"PublisherUrl: https://github.com/hhasanguclu\n"
        "PackageName: kazma\n"
        f"PackageUrl: https://github.com/{REPO}\n"
        "License: MIT\n"
        f"LicenseUrl: https://github.com/{REPO}/blob/main/LICENSE\n"
        f"ShortDescription: {DESC}\n"
        "Moniker: kz\n"
        "Tags:\n- cleaner\n- disk-usage\n- uninstaller\n- drivers\n- tui\n"
        f"ReleaseNotesUrl: https://github.com/{REPO}/releases/tag/v{version}\n"
        "ManifestType: defaultLocale\nManifestVersion: 1.6.0\n",
        f"{ID}.installer.yaml": head
        + f"PackageIdentifier: {ID}\nPackageVersion: {version}\n"
        "InstallerType: portable\nCommands:\n- kz\n"
        "MinimumOSVersion: 10.0.17763.0\n"
        "Installers:\n"
        f"- Architecture: x64\n  InstallerUrl: {url}\n  InstallerSha256: {sha.upper()}\n"
        "ManifestType: installer\nManifestVersion: 1.6.0\n",
    }


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    version = sys.argv[1].lstrip("vV")
    sha = sys.argv[2].strip().lower()
    if not re.fullmatch(r"\d+\.\d+\.\d+", version) or not re.fullmatch(r"[0-9a-f]{64}", sha):
        print("bad version or sha256", file=sys.stderr)
        return 2

    bucket = ROOT / "bucket" / "kazma.json"
    bucket.parent.mkdir(parents=True, exist_ok=True)
    bucket.write_text(json.dumps(scoop(version, sha), indent=4) + "\n", encoding="utf-8")

    out = ROOT / "packaging" / "winget" / version
    out.mkdir(parents=True, exist_ok=True)
    for name, text in winget(version, sha).items():
        (out / name).write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote {bucket.relative_to(ROOT)} and {out.relative_to(ROOT)}/")
    return 0


if __name__ == "__main__":
    sys.exit(main())
