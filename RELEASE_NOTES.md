```
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
```

Oldschool, all-ASCII system cleaner, disk explorer and system monitor for Windows 10/11 (x64).
One ~1.5 MB exe. No installer, no admin rights, no telemetry.

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
kz                  interactive menu
kz clean            caches, temp files, logs, Recycle Bin
kz analyze [path]   disk usage explorer
kz status           live cpu / memory / disk / network monitor
kz purge [paths]    node_modules, target, bin/obj, .venv ...
kz history          what kazma has removed
```

Every command that deletes supports `--dry-run` (`-n`) to preview first.
Run `kz clean` from an **elevated** terminal to also clean Windows Update and system temp caches.

## Uninstall

```powershell
kz remove
```

or, without kazma on PATH:

```powershell
iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/uninstall.ps1 | iex
```

If you only downloaded `kz.exe`, just delete it (and `%LOCALAPPDATA%\kazma`, which holds the operation log).

## Safety

- Only known, regenerable caches are cleaned; temp files younger than 24h are kept.
- `purge` needs a project marker next to the folder (`package.json`, `Cargo.toml`, `.csproj`...).
- Drive roots, your profile, `Windows`, `Program Files` and `AppData` are hard-blocked.
- `analyze` deletes to the Recycle Bin. Every deletion is logged.
