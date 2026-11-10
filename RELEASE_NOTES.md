```
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
```

Oldschool, all-ASCII system cleaner, disk explorer and system toolkit for Windows 10/11 (x64).
One ~1.5 MB exe. No installer required, no telemetry.

## What's new in v0.2.0

- **`kz uninstall`**: remove programs with their own uninstaller, then send leftover folders to the Recycle Bin
- **`kz upgrade`**: update installed programs through winget, picked from a checklist
- **`kz drivers`**: driver updates from Windows Update, with a restore point first; `--installed` shows driver ages
- **`kz optimize`**: DNS flush, icon cache refresh, SSD TRIM, DISM cleanup, SFC
- **`kz installer`**: find old setup files (`.exe`, `.msi`, `.iso`...) in Downloads and on the Desktop
- **`kz whitelist`**: paths that kazma must never delete
- **`kz remove`**: uninstall kazma itself
- Icon and version info in `kz.exe`, install / uninstall scripts

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

**Manual**: download `kz.exe` above and run it from a terminal (Windows Terminal recommended).

## Usage

```
kz                       interactive menu
kz clean                 caches, temp files, logs, Recycle Bin
kz analyze [path]        disk usage explorer
kz status                live cpu / memory / disk / network monitor
kz purge [paths]         node_modules, target, bin/obj, .venv ...
kz installer             old setup files in Downloads / Desktop
kz uninstall             remove programs + leftovers
kz upgrade               update installed programs (winget)
kz drivers               driver updates from Windows Update
kz drivers --installed   installed drivers, oldest first
kz optimize              dns flush, icon cache, ssd trim, dism, sfc
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
