```
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
```

Oldschool, all-ASCII system cleaner, disk explorer and system toolkit for Windows 10/11 (x64).
One ~2.7 MB exe. No installer required, no telemetry.

## What's new in v0.4.0

- **`kz backup`**: move-to-a-new-PC kit (winget app list, drivers, program list, settings); `kz backup restore FOLDER` reinstalls
- **`kz status`**: now a process manager: select a process, `x` ends it, `m` sorts by memory, `/` searches
- **`kz ports`**: which program listens on which port; `--kill PORT`
- **`kz net`**: IP and latency, DNS benchmark and `dns set cloudflare|google|quad9|adguard|opendns|auto`, `speed` test, saved `wifi`
- **`kz empty`**: empty folders, broken shortcuts, 0-byte files
- **`kz power`**: switch power plans, battery report, what wakes the PC
- **`kz report`**: one HTML file with hardware, health, startup, programs and drivers
- **`kz restore`**: list / create System Restore points, open the restore wizard
- **`kz context install`**: "Analyze with kazma" in Explorer's right-click menu
- **`kz bench`**: disk speed test (sequential + 4K random, OS cache bypassed)
- **`kz rules`**: your own folders for `kz clean`
- **Every list**: `/` search and mouse support; `analyze` sorts by size / name / date with `s`
- **Menu**: tells you when a new version is out

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
kz analyze [path]        disk explorer (b = largest files, s = sort, / = search)
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
