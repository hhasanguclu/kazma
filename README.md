<div align="center">

```
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
```

**Oldschool, all-ASCII system toolkit for Windows**

[![release](https://img.shields.io/github/v/release/hhasanguclu/kazma?style=flat-square)](https://github.com/hhasanguclu/kazma/releases/latest)
[![ci](https://img.shields.io/github/actions/workflow/status/hhasanguclu/kazma/ci.yml?branch=main&style=flat-square&label=windows%20ci)](https://github.com/hhasanguclu/kazma/actions)
[![license](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
![platform](https://img.shields.io/badge/windows-10%20%7C%2011-blue?style=flat-square)

**English** · [Türkçe](README.tr.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [Polski](README.pl.md) · [Українська](README.uk.md) · [Русский](README.ru.md)

<img src="docs/screenshots/menu-en.png" width="49%" alt="kazma main menu in English"> <img src="docs/screenshots/menu-ja.png" width="49%" alt="kazma main menu in Japanese, blue theme">

</div>

---

**kazma** (Turkish for *pickaxe*) is a single ~5 MB `kz.exe` that cleans, explores and maintains a
Windows PC from the terminal, with a retro green-screen look. No installer, no background service,
no telemetry. The interface is available in 11 languages (English, Türkçe, 简体中文, 繁體中文, 日本語, 한국어, Deutsch, Français, Polski,
Українська, Русский) and starts in your Windows display language; change it in `kz config` (arrow keys) or with `kz config lang de`.

## Features

| | |
|---|---|
| **Clean up** | 100+ kinds of caches, temp files, logs and the Recycle Bin, including AI tools, game launchers and GPU driver leftovers (`clean`) · Windows.old, upgrade files, memory dumps, hibernation file, old restore points (`deepclean`) · disk explorer with treemap and largest-files view (`analyze`) · file types (`types`) · project build artifacts (`purge`) · old installers (`installer`) · duplicate files (`dupes`) · empty folders and broken shortcuts (`empty`) · usage traces (`privacy`) |
| **Programs & drivers** | uninstall programs and their leftovers (`uninstall`) · update programs with winget (`upgrade`) · driver updates from Windows Update (`drivers`) · startup programs (`startup`) · promo apps and Windows ads (`debloat`) · move-to-a-new-PC backup (`backup`) |
| **System** | health report (`health`) · live monitor with GPU and battery, process manager that explains why a process runs (`status`) · maintenance: DNS, icon cache, SSD TRIM, DISM, SFC (`optimize`) · repairs: network, Windows Update, Store, system files (`repair`) · power plans and battery (`power`) · restore points (`restore`) · HTML report (`report`) · disk speed test (`bench`) · automatic cleaning (`schedule`) |
| **Network** | IP, latency, DNS benchmark and switch, speed test, saved Wi-Fi (`net`) · who uses which port (`ports`) |
| **Handy** | which program holds a file open (`lock`) · keep the screen awake (`awake`) · wipe-the-screen mode (`cleanscreen`) |
| **kazma** | protected paths (`whitelist`) · your own clean rules (`rules`) · Explorer right-click menu (`context`) · language and theme (`config`) · self-update (`update`) · log (`history`) · uninstall kazma (`remove`) |

Every list has `/` search and works with the mouse (click to select, click again to open, wheel to scroll).
The menu tells you when a new version is out.

It is fast. Run as administrator on an NTFS drive, `analyze` and `types` read the drive's Master File Table
directly (the trick behind WizTree), so the sizes of a whole drive are ready in seconds. Otherwise sizes are
computed in parallel, and `dupes` compares by size, then the first and last
64 KB, and only then hashes whole files with BLAKE3 (83,000 files / 5.9 GB in about 7 seconds).

## Install

Windows 10/11, x64 and ARM64. No admin rights needed.

```powershell
iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/install.ps1 | iex
```

This installs to `%LOCALAPPDATA%\Programs\kazma` and adds `kz` to your PATH. Alternatives:

- **Scoop**: `scoop bucket add kazma https://github.com/hhasanguclu/kazma` then `scoop install kazma`
- **winget**: manifests for every release are generated in [`packaging/winget`](packaging/winget); once accepted: `winget install hhasanguclu.kazma`
- **Manual**: download `kz.exe` from the [latest release](https://github.com/hhasanguclu/kazma/releases/latest)

> The exe is not code-signed yet, so SmartScreen may warn: **More info -> Run anyway**.
> Signing via SignPath is prepared, see [docs/SIGNING.md](docs/SIGNING.md).

**Uninstall**: `kz remove` (removes the exe, its PATH entry and `%LOCALAPPDATA%\kazma`).

## Commands

| Command | What it does |
|---|---|
| `kz` | Interactive menu |
| `kz clean` | Temp files, crash dumps, browser / app / developer caches, Recycle Bin |
| `kz deepclean` | Windows.old, upgrade setup files, ESD files, memory dumps, hibernation file, older restore points (admin) |
| `kz analyze [path]` | Disk usage explorer: `t` treemap, `b` largest files, `s` sort, `/` search, `w` who uses it; explains special files (hiberfil, WSL disks, iPhone backups...); delete goes to the Recycle Bin |
| `kz types [path]` | What kind of files take the space (video, images, archives...) and the top extensions |
| `kz purge [paths...]` | Build artifacts: `node_modules`, `target`, `bin/obj`, `.venv`, `__pycache__`, ... |
| `kz installer [paths...]` | Old setup files (`.exe`, `.msi`, `.msix`, `.iso`) in Downloads and on the Desktop |
| `kz dupes [paths...]` | Duplicate files (size -> partial hash -> BLAKE3, all parallel) |
| `kz empty [paths...]` | Empty folders, broken shortcuts and 0-byte files (to the Recycle Bin) |
| `kz privacy` | Recent files, Run / Explorer history, clipboard, browser history and cookies (opt-in) |
| `kz uninstall` | Remove classic, Microsoft Store and Scoop apps, then leftover folders and registry keys (backed up first); `--leftovers` finds remains of apps removed some other way |
| `kz upgrade` | Update installed programs through winget |
| `kz drivers` | Driver updates from Windows Update; `--installed` lists third-party drivers by age |
| `kz startup` | Turn startup programs on/off (same flags as Task Manager, fully reversible) |
| `kz debloat` | Remove preinstalled promo apps from a fixed list, turn off ads; `--restore` undoes |
| `kz backup [folder]` | New-PC kit: winget app list, drivers, program list, kazma settings; `kz backup restore FOLDER` reinstalls |
| `kz health` | Disk SMART, battery wear, blue screens, errors, updates, Defender, firewall |
| `kz status` | Live CPU, GPU, memory, disk, network, battery; `enter` explains why a process runs, `x` ends it, `m` sorts by memory |
| `kz power` | Power plans (`set saver / balanced / high / ultimate`), `battery` report, `wake` sources, `hibernate off / reduced / full` |
| `kz restore` | System Restore points: `list`, `create "desc"`, `enable`, `open` (Windows' own wizard) |
| `kz report` | HTML system report: hardware, health, startup, programs, drivers |
| `kz bench [path]` | Disk speed test: sequential and 4K random, OS cache bypassed |
| `kz net` | IP and latency; `dns` benchmark, `dns set cloudflare`; `speed` test; `wifi [--show]` |
| `kz ports [filter]` | Which program listens on which port; `--kill PORT` closes it |
| `kz optimize` | Broken startup entries (backed up), restart Start/Search, DNS flush, icon cache, SSD TRIM / DISM / SFC when due |
| `kz repair [fix]` | `network`, `update` (Windows Update reset), `store`, `dism` (+SFC), `time`, `printer`; restore point first (admin) |
| `kz schedule weekly` | Run `kz clean --yes` automatically (`daily`, `weekly`, `monthly`, `off`, `status`) |
| `kz lock <path>` | Which program has a file or folder open |
| `kz awake [minutes]` | Keep the screen and PC awake |
| `kz cleanscreen [seconds]` | Black screen that ignores input, to wipe the display |
| `kz whitelist` | Paths kazma must never delete (`--add`, `--remove`, `--edit`) |
| `kz rules` | Your own clean rules for `kz clean` (`--edit` opens Notepad) |
| `kz context install` | "Analyze with kazma" in Explorer's right-click menu (`uninstall` removes it) |
| `kz config` | Pick language and theme with the arrow keys; or `kz config lang de` (auto, en, tr, zh-CN, zh-TW, ja, ko, de, fr, pl, uk, ru), `kz config theme amber` (green, amber, white, blue) |
| `kz update` | Update kazma itself from GitHub (checksum verified) |
| `kz history` | Log of everything kazma did |
| `kz remove` | Uninstall kazma |

Every command that deletes or installs supports `--dry-run` (`-n`) to preview first.
Run from an **elevated** terminal (Run as administrator) to install drivers, run admin maintenance
tasks, and clean Windows Update and system temp caches.

## Safety

- `clean` only touches known, regenerable cache and temp locations; temp files younger than 24 h are kept.
- `purge` only deletes a folder when a project marker sits next to it (`package.json`, `Cargo.toml`, `.csproj`...).
- Drive roots, your profile, `Windows`, `Program Files`, `AppData` and similar folders are hard-blocked.
- `analyze`, `installer`, `dupes` and `uninstall` leftovers go to the **Recycle Bin**.
- `dupes` always keeps at least one file of every group.
- `drivers` only installs drivers offered by **Windows Update** and creates a **restore point** first.
- `startup` only flips Task Manager's on/off flags; `debloat` only removes apps on a fixed list and `--restore` undoes its settings.
- `update` verifies the download against the release's `SHA256SUMS.txt`.
- `deepclean` removes Windows.old and upgrade files through Windows' own Disk Cleanup, and always keeps the newest restore point.
- `repair` renames the Windows Update folders instead of deleting them and offers a restore point first.
- The MFT reader only reads the drive; it never writes to it.
- Registry leftovers and broken startup entries are exported to `%LOCALAPPDATA%\kazma\backups` before they are removed.
- `uninstall --leftovers` suggestions start unselected and go to the Recycle Bin.
- **Whitelist**: paths in `%LOCALAPPDATA%\kazma\whitelist.txt` are never deleted. If that file exists but can't be read, kazma refuses to run.
- Every action is logged (`kz history`); without a terminal nothing is deleted unless `--yes` is passed.

## Build

```bash
cargo build --release                                  # on Windows -> target\release\kz.exe
rustup target add x86_64-pc-windows-gnu                # cross-compile from Linux
sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
```

The code also builds on Linux/macOS for development; Windows-only parts are behind `cfg(windows)`.
To release, bump `version` in `Cargo.toml` and push to `main`: CI tests on Windows, runs every read-only
command for real, then publishes `vX.Y.Z` with `kz.exe`, a zip and `SHA256SUMS.txt`, and updates the
Scoop / winget manifests.

## Screenshots

| | |
|---|---|
| **`kz`** · English<br><img src="docs/screenshots/menu-en.png" alt="kz main menu, English"> | **`kz`** · 日本語<br><img src="docs/screenshots/menu-ja.png" alt="kz main menu, Japanese, blue theme"> |
| **`kz`** · Türkçe<br><img src="docs/screenshots/menu-tr.png" alt="kz main menu, Turkish, amber theme"> | **`kz analyze`** · `(i)`<br><img src="docs/screenshots/analyze-tr.png" alt="kz analyze"> |
| **`kz analyze`** + `b`<br><img src="docs/screenshots/analyze-largest.png" alt="kz analyze, largest files"> | **`kz analyze`** + `t`<br><img src="docs/screenshots/treemap-tr.png" alt="kz analyze, treemap"> |
| **`kz clean`** · Türkçe<br><img src="docs/screenshots/clean-tr.png" alt="kz clean, Turkish"> | **`kz clean`** · Deutsch<br><img src="docs/screenshots/clean-de.png" alt="kz clean, German, white theme"> |
| **`kz types`**<br><img src="docs/screenshots/types-en.png" alt="kz types"> | **`kz purge`**<br><img src="docs/screenshots/purge-en.png" alt="kz purge, blue theme"> |
| **`kz health`** · Windows<br><img src="docs/screenshots/health-windows.png" alt="kz health on Windows"> | **`kz health`** · Windows · 日本語<br><img src="docs/screenshots/health-windows-ja.png" alt="kz health on Windows, Japanese"> |
| **`kz startup --list`** · Windows<br><img src="docs/screenshots/startup-windows.png" alt="kz startup on Windows"> |  |

<sub>`health` and `startup` were captured from a real Windows run (GitHub Actions, Windows Server 2025).
The other screens come from the same code running in a terminal with demo data, which is why their paths
look like `/home/user/demo`.</sub>

## Credits

**Inspiration.** kazma was inspired by [Mole](https://github.com/tw93/Mole) by
[tw93](https://github.com/tw93), a deep-clean toolkit for macOS. kazma is written from scratch in Rust and
shares no code with it.

**Translations.** Every language other than English and Turkish lives in [`locales/`](locales) as a JSON file
that maps the English text to the translation. Corrections are very welcome: edit the file and open a pull request
(`python tools/i18n_extract.py --missing de` lists what a language still lacks).

**Libraries.** Thanks to the authors and maintainers of:

| Crate | Used for | License |
|---|---|---|
| [ratatui](https://github.com/ratatui/ratatui) | terminal UI | MIT |
| [crossterm](https://github.com/crossterm-rs/crossterm) | terminal input & colors | MIT |
| [clap](https://github.com/clap-rs/clap) | command line | MIT / Apache-2.0 |
| [sysinfo](https://github.com/GuillaumeGomez/sysinfo) | CPU, memory, disks, processes | MIT |
| [rayon](https://github.com/rayon-rs/rayon) | parallel scanning | MIT / Apache-2.0 |
| [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) | duplicate hashing | CC0 / Apache-2.0 |
| [trash](https://github.com/Byron/trash-rs) | Recycle Bin | MIT |
| [serde](https://github.com/serde-rs/serde), [serde_json](https://github.com/serde-rs/json) | PowerShell JSON, translations | MIT / Apache-2.0 |
| [anyhow](https://github.com/dtolnay/anyhow) | errors | MIT / Apache-2.0 |
| [syn](https://github.com/dtolnay/syn), [quote](https://github.com/dtolnay/quote) | the `tr!` translation macro | MIT / Apache-2.0 |
| [windows-sys](https://github.com/microsoft/windows-rs) | Windows API | MIT / Apache-2.0 |
| [winresource](https://github.com/BenjaminRi/winresource) | exe icon & version info | MIT |

**Platform.** Microsoft's [winget](https://github.com/microsoft/winget-cli) (program updates), the Windows Update
Agent API (driver updates), [Scoop](https://scoop.sh) (distribution) and the
[SignPath Foundation](https://signpath.org) (free code signing for open source).

**Art.** The banner uses the classic [FIGlet](http://www.figlet.org) "standard" letter style; screenshots
are rendered with the [DejaVu Sans Mono](https://dejavu-fonts.github.io) font.

**Author.** [hhasanguclu](https://github.com/hhasanguclu). Bug reports, ideas and pull requests are welcome.

## License

[MIT](LICENSE)
