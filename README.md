```
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
```

**kazma** is an oldschool, all-ASCII system cleaner and disk explorer for Windows.
One small native binary (`kz.exe`), no installer, no telemetry.

```
+[ kazma analyze ]-------------------------------------------------------+
| C:\Users\me  total 84.2 GB                                             |
+------------------------------------------------------------------------+
+[ 14 items ]------------------------------------------------------------+
|>[####################]    31.0 GB  36.8% AppData\                     |
| [###########.........]    17.4 GB  20.7% source\                      |
| [#######.............]    11.2 GB  13.3% Downloads\                   |
| [###.................]     5.1 GB   6.1% Videos\                      |
+------------------------------------------------------------------------+
```

## Install

Windows 10/11 x64. No admin rights needed.

**PowerShell one-liner**: installs to `%LOCALAPPDATA%\Programs\kazma` and adds `kz` to your PATH:

```powershell
iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/install.ps1 | iex
```

**Manual**: grab `kz.exe` from the [latest release](https://github.com/hhasanguclu/kazma/releases/latest) and run it from a terminal.

> The exe is not code-signed yet, so SmartScreen may warn: **More info -> Run anyway**.

## Uninstall

```powershell
kz remove
```

or `iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/uninstall.ps1 | iex`.
This removes the exe, its PATH entry and the log folder `%LOCALAPPDATA%\kazma`.

## Commands

| Command | What it does |
|---|---|
| `kz` | Interactive menu |
| `kz clean` | Temp files, crash dumps, browser / app / developer caches, Recycle Bin |
| `kz analyze [path]` | Interactive disk usage explorer; delete goes to the Recycle Bin |
| `kz status` | Live CPU, memory, disk, network and top-process dashboard |
| `kz purge [paths...]` | Build artifacts: `node_modules`, `target`, `bin/obj`, `.venv`, `__pycache__`, ... |
| `kz installer [paths...]` | Old setup files (`.exe`, `.msi`, `.msix`, `.iso`) in Downloads and on the Desktop |
| `kz uninstall` | Remove programs with their own uninstaller, then clean up leftover folders |
| `kz upgrade` | Update installed programs through winget (pick from a checklist, silent install) |
| `kz drivers` | Driver updates from Windows Update; `--installed` lists third-party drivers by age |
| `kz optimize` | DNS flush, icon cache refresh, SSD TRIM, DISM component cleanup, SFC |
| `kz whitelist` | Paths kazma must never delete (`--add`, `--remove`, `--edit`) |
| `kz history` | Log of everything kazma did |
| `kz remove` | Uninstall kazma |

Every deleting command supports `--dry-run` (`-n`) to preview and `--yes` (`-y`) to skip the selection screen.

## Safety

- `clean` only touches known, regenerable cache and temp locations. Temp folders only lose files older than 24 hours.
- `purge` only deletes a folder when a project marker sits next to it (`node_modules` needs `package.json`,
  `target` needs `Cargo.toml`, `bin/obj` need a `.csproj`, ...). Projects touched in the last 7 days are unselected by default.
- Drive roots, the user profile, `Windows`, `Program Files`, `AppData` and similar folders are hard-blocked.
- Symlinks and junctions are never followed; locked files are skipped, not forced.
- System locations (Windows Update cache, `C:\Windows\Temp`, ...) are only offered when run as administrator.
- Without a terminal (scripts, pipes) nothing is deleted unless `--yes` is passed.
- `drivers` only installs drivers offered by **Windows Update** (Microsoft-signed, matched to your exact
  hardware IDs), never from third-party download sites, and creates a **system restore point** first.
- `upgrade` uses **winget**, the official Windows package manager, so installers come from each vendor's own source.
- `installer`, `uninstall` leftovers and `analyze` deletions go to the **Recycle Bin**, so they can be restored.
- `uninstall` runs each program's own uninstaller and only offers leftover folders whose name exactly matches
  the program, never shared vendor folders such as `Microsoft` or `Google`.
- **Whitelist**: anything listed in `%LOCALAPPDATA%\kazma\whitelist.txt` is never deleted by any command.
  If that file exists but cannot be read, kazma refuses to run.
- Every deletion is appended to `%LOCALAPPDATA%\kazma\operations.log`. Set `KAZMA_NO_OPLOG=1` to disable.

## Build

```bash
cargo build --release          # on Windows -> target\release\kz.exe
```

Cross-compiling from Linux:

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
```

The code also builds on Linux/macOS so it can be developed and tested anywhere; Windows-only parts live in `src/sys.rs`.

## Releasing

Bump `version` in `Cargo.toml` and push to `main`. CI builds and tests on Windows, then creates the
`vX.Y.Z` tag and a GitHub release with `kz.exe`, a zip and `SHA256SUMS.txt`, using `RELEASE_NOTES.md` as the text.

## Layout

```
src/
  main.rs     CLI + main menu
  clean.rs    cleanup targets
  purge.rs    build artifact finder
  analyze.rs  disk explorer TUI
  status.rs   system dashboard TUI
  fsutil.rs   parallel sizing, guarded deletion
  sys.rs      Windows API layer (Recycle Bin, admin check)
  ui.rs       ASCII theme, banner, checklist widget
  oplog.rs    operation log
  remove.rs   self-uninstall
  upgrade.rs  winget program updates
  drivers.rs  Windows Update driver updates
  ps.rs       PowerShell helpers
  installer.rs  installer file finder
  uninstall.rs  program uninstaller + leftovers
  optimize.rs   maintenance tasks
  whitelist.rs  protected paths
```

## License

MIT
