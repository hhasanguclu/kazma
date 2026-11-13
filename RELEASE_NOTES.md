```
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
```

Oldschool, all-ASCII system cleaner, disk explorer and system toolkit for Windows 10/11 (x64 and ARM64).
One ~5 MB exe. No installer required, no telemetry.

## What's new in v1.0.3

Found by the new interface check on real Windows, in all 11 languages:

- `clean`: the "would free" summary box keeps its right edge in Chinese, Japanese and Korean
- `power`: known power plans are shown in your language, not only in the language of Windows
- `startup --list`, `optimize`, `repair`, `health`: long command lines and descriptions are cut to
  the terminal width instead of wrapping (`COLUMNS` sets a fixed width for scripts)
- The interface check now blocks a release if any language has a layout problem

## v1.0.2

Interface audit in all 11 languages: every command and screen was captured at 100x32 and 80x24
and checked automatically (box edges, column alignment with CJK counted as two columns, leftover
English, overflow). Fixed:

- Text columns line up in every language: labels are measured by display width, and tables size
  their name column to the longest name (optimize, clean, privacy, deepclean, repair, restore,
  power, startup, net, bench, types, status)
- Treemap: Chinese / Japanese / Korean file names no longer push the box edges out of line
- status: long paths and command lines in "why is this running?" wrap under the value column
- Yes/no questions accept "yes" in every language (ja, oui, tak, да, так, 是, はい, 네...)
- CI now runs the read-only commands on Windows in all 11 languages and checks their layout

## v1.0.1

- **`kz config` is now a screen**: pick the language and the color theme with the arrow keys
  (or a mouse click). A choice is saved at once and the screen switches to it right away.
  `kz config lang de` still works for scripts.
- Commands started from the main menu open on a clean screen instead of under the previous output.
- `kz history` got the same header as the other commands.
- `kz clean`: the group column fits long group names in every language.
- `kz health`: "Firewall" is translated.

## v1.0.0

**11 languages.** English, Türkçe, 简体中文, 繁體中文, 日本語, 한국어, Deutsch, Français, Polski, Українська, Русский.
kazma starts in your Windows display language; change it with `kz config lang de` (or `auto`).
Translations live in `locales/*.json`, corrections are welcome. The README is available in every language.

**Mole parity:**

- **100+ cleanup rules**: AI tools (Claude, ChatGPT, Cursor, Windsurf, LM Studio; model caches opt-in),
  game launchers (Steam, Epic, EA, Battle.net, Ubisoft, Riot, GOG, Xbox), GPU driver leftovers
  (NVIDIA, AMD, Intel), dev tools (JetBrains, VS, bun, deno, uv, poetry, cargo, rustup...) and Electron apps
- **analyze**: `(i)` explains special files (hiberfil, pagefile, WSL/Docker disks, iPhone backups,
  WinSxS...); `w` shows which program holds the selected item open
- **`kz lock <path>`**: which program has a file or folder open (Restart Manager)
- **status**: GPU usage and memory, battery; `enter` explains *why a process is running*
  (service, startup entry, parent)
- **`kz awake [minutes]`** keeps the PC awake; **`kz cleanscreen [seconds]`** black screen for wiping the display
- **uninstall**: Microsoft Store and Scoop apps, registry leftovers (exported to a .reg backup first),
  `--leftovers` finds folders of apps removed some other way
- **optimize**: remove broken startup entries (backed up), restart Start menu and Search,
  TRIM / DISM / SFC pre-selected only when due
- **ARM64 build** (`kz-arm64.exe`); installer, `kz update`, Scoop and winget pick the right one

**New ASCII interface**, clearer and richer while staying minimal:

- **Main menu in two columns**: all commands on one screen, dimmed descriptions, group headings,
  a detail bar for the selected command, left/right moves between columns
- **Calmer look**: soft ASCII corners (`.---.` / `'---'`), quieter frames, a one-color selection bar,
  a gradient banner in every theme
- **Bars** show the filled part in color and the rest in the quiet line color, everywhere
- **Key hints** with keys in the accent color; **compact three-line header** for every command
- **Checklists**: unchecked rows step back, a clear "n of m selected" summary
- **analyze**: name first, then size, percent and bar
- **status**: no more noise in the network graph when idle

## v0.5.0

- **Instant drive scans**: run as administrator on NTFS and `kz analyze` / `kz types` read the Master File Table
  directly (the trick behind WizTree). Verified byte-for-byte against a normal scan in CI.
- **`kz analyze`**: `t` shows an ASCII **treemap** of the current folder
- **`kz types`**: what kind of files take the space (video, images, archives, programs...) and the top extensions
- **`kz deepclean`**: Windows.old, upgrade setup files, ESD files, memory dumps, hibernation file (shrink),
  older restore points (newest kept). Uses Windows' own Disk Cleanup for the protected folders.
- **`kz repair`**: reset network (Winsock, TCP/IP, DNS), reset Windows Update components, reset Store cache,
  DISM RestoreHealth + SFC, sync the clock, clear a stuck print queue. Restore point first.
- **`kz power hibernate off | reduced | full`**

## Download

| File | What it is |
|---|---|
| **`kz.exe`** | The program. Download and run it, or put it somewhere on your PATH. |
| `kazma-windows-x64.zip` | `kz.exe` + README + LICENSE |
| `SHA256SUMS.txt` | Checksums to verify the downloads |

> Windows SmartScreen may warn because the exe is not code-signed yet. Click **More info -> Run anyway**.

## Install

**One-liner (PowerShell)**: installs to `%LOCALAPPDATA%\Programs\kazma` and adds `kz` to your PATH:

```powershell
iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/install.ps1 | iex
```

**Scoop**:

```powershell
scoop bucket add kazma https://github.com/hhasanguclu/kazma
scoop install kazma
```

**Manual**: download `kz.exe` above and run it from a terminal (Windows Terminal recommended).

## Usage

```
kz                       interactive menu (English / Türkçe, mouse works)
kz clean                 caches, temp files, logs, Recycle Bin
kz deepclean             Windows.old, upgrade files, dumps, hibernation file
kz analyze [path]        disk explorer (t = treemap, b = largest files, s = sort, / = search)
kz types [path]          what kind of files take the space
kz purge [paths]         node_modules, target, bin/obj, .venv ...
kz installer             old setup files in Downloads / Desktop
kz dupes [paths]         duplicate files
kz empty [paths]         empty folders, broken shortcuts, 0-byte files
kz privacy               usage traces
kz uninstall             remove programs + leftovers
kz upgrade               update installed programs (winget)
kz drivers               driver updates from Windows Update
kz startup               startup programs on / off
kz debloat               promo apps and Windows ads
kz backup [folder]       new-PC kit;  kz backup restore FOLDER
kz health                system health report
kz status                live monitor + process manager
kz optimize              dns flush, icon cache, ssd trim, dism, sfc
kz repair [fix]          network, update, store, dism, time, printer
kz power                 power plans, battery report, wake sources
kz restore               System Restore points
kz report                HTML system report
kz bench [path]          disk speed test
kz schedule weekly       automatic cleaning
kz net                   ip, latency, dns, speed test, wifi
kz ports [filter]        who listens on which port
kz rules                 your own clean rules
kz context install       Explorer right-click menu
kz config lang tr        language / theme
kz update                update kazma
kz whitelist             paths kazma must never delete
kz history               what kazma has done
```

Every command that deletes or installs supports `--dry-run` (`-n`) to preview first.
Run from an **elevated** terminal (Run as administrator) to install drivers, run the admin
maintenance tasks, and clean Windows Update and system temp caches.

## Uninstall

```powershell
kz remove
```

or, without kazma on PATH:

```powershell
iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/uninstall.ps1 | iex
```

If you only downloaded `kz.exe`, just delete it (and `%LOCALAPPDATA%\kazma`, which holds the log and whitelist).

## Safety

- Only known, regenerable caches are cleaned; temp files younger than 24h are kept.
- `purge` needs a project marker next to the folder (`package.json`, `Cargo.toml`, `.csproj`...).
- Drive roots, your profile, `Windows`, `Program Files` and `AppData` are hard-blocked.
- `installer`, `uninstall` leftovers and `analyze` send files to the Recycle Bin.
- Drivers only come from Windows Update, never from third-party sites.
- Paths in the whitelist are never touched. Every action is logged (`kz history`).
