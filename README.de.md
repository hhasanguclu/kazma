<div align="center">

```
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
```

**Oldschool-Systemtoolkit für Windows, komplett in ASCII**

[![release](https://img.shields.io/github/v/release/hhasanguclu/kazma?style=flat-square)](https://github.com/hhasanguclu/kazma/releases/latest)
[![ci](https://img.shields.io/github/actions/workflow/status/hhasanguclu/kazma/ci.yml?branch=main&style=flat-square&label=windows%20ci)](https://github.com/hhasanguclu/kazma/actions)
[![license](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
![platform](https://img.shields.io/badge/windows-10%20%7C%2011-blue?style=flat-square)

[English](README.md) · [Türkçe](README.tr.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · **Deutsch** · [Français](README.fr.md) · [Polski](README.pl.md) · [Українська](README.uk.md) · [Русский](README.ru.md)

<img src="docs/screenshots/menu-tr.png" width="49%" alt="kazma-Hauptmenü auf Türkisch"> <img src="docs/screenshots/menu-en.png" width="49%" alt="kazma-Hauptmenü auf Englisch, Bernstein-Theme">

</div>

---

**kazma** (türkisch für *Spitzhacke*) ist eine einzelne, ca. 2,7 MB große `kz.exe`, die einen Windows-PC
vom Terminal aus bereinigt, durchleuchtet und wartet – im Retro-Look eines grünen Monitors. Kein Installer,
kein Hintergrunddienst, keine Telemetrie. Die Oberfläche gibt es in 11 Sprachen (English, Türkçe, 简体中文, 繁體中文, 日本語, 한국어, Deutsch, Français, Polski,
Українська, Русский); sie startet in der Anzeigesprache von Windows und lässt sich mit `kz config lang de` ändern.

## Funktionen

| | |
|---|---|
| **Aufräumen** | über 100 Arten von Caches, temporären Dateien, Protokollen und der Papierkorb, inklusive KI-Tools, Spiele-Launchern und Resten von GPU-Treibern (`clean`) · Windows.old, Upgradedateien, Speicherabbilder, Ruhezustandsdatei, alte Wiederherstellungspunkte (`deepclean`) · Datenträger-Explorer mit Treemap und Ansicht der größten Dateien (`analyze`) · Dateitypen (`types`) · Build-Artefakte von Projekten (`purge`) · alte Installer (`installer`) · doppelte Dateien (`dupes`) · leere Ordner und defekte Verknüpfungen (`empty`) · Nutzungsspuren (`privacy`) |
| **Programme & Treiber** | Programme samt Resten deinstallieren (`uninstall`) · Programme per winget aktualisieren (`upgrade`) · Treiberupdates über Windows Update (`drivers`) · Autostart-Apps (`startup`) · Werbe-Apps und Windows-Werbung (`debloat`) · Sicherung für den Umzug auf einen neuen PC (`backup`) |
| **System** | Zustandsbericht (`health`) · Live-Monitor mit GPU und Akku, Prozessmanager, der erklärt, warum ein Prozess läuft (`status`) · Wartung: DNS, Symbolcache, SSD-TRIM, DISM, SFC (`optimize`) · Reparaturen: Netzwerk, Windows Update, Store, Systemdateien (`repair`) · Energiesparpläne und Akku (`power`) · Wiederherstellungspunkte (`restore`) · HTML-Bericht (`report`) · Datenträger-Geschwindigkeitstest (`bench`) · automatische Bereinigung (`schedule`) |
| **Netzwerk** | IP, Latenz, DNS-Benchmark und -Wechsel, Speedtest, gespeicherte WLANs (`net`) · wer welchen Port nutzt (`ports`) |
| **Praktisch** | welches Programm eine Datei geöffnet hält (`lock`) · Bildschirm wach halten (`awake`) · Modus zum Abwischen des Bildschirms (`cleanscreen`) |
| **kazma** | geschützte Pfade (`whitelist`) · eigene Bereinigungsregeln (`rules`) · Rechtsklickmenü im Explorer (`context`) · Sprache und Theme (`config`) · Selbstaktualisierung (`update`) · Protokoll (`history`) · kazma deinstallieren (`remove`) |

Jede Liste hat eine `/`-Suche und funktioniert mit der Maus (Klick wählt aus, erneuter Klick öffnet, Mausrad scrollt).
Das Menü meldet, wenn eine neue Version erschienen ist.

Es ist schnell. Als Administrator auf einem NTFS-Laufwerk lesen `analyze` und `types` direkt die Master File Table
des Laufwerks (der Trick hinter WizTree), sodass die Größen eines ganzen Laufwerks in Sekunden vorliegen. Andernfalls
werden die Größen parallel berechnet, und `dupes` vergleicht zuerst nach Größe, dann die ersten und letzten
64 KB und hasht erst danach ganze Dateien mit BLAKE3 (83.000 Dateien / 5,9 GB in etwa 7 Sekunden).

## Installation

Windows 10/11, x64 und ARM64. Keine Administratorrechte nötig.

```powershell
iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/install.ps1 | iex
```

Das installiert nach `%LOCALAPPDATA%\Programs\kazma` und fügt `kz` zum PATH hinzu. Alternativen:

- **Scoop**: `scoop bucket add kazma https://github.com/hhasanguclu/kazma`, dann `scoop install kazma`
- **winget**: Manifeste für jedes Release werden in [`packaging/winget`](packaging/winget) erzeugt; sobald angenommen: `winget install hhasanguclu.kazma`
- **Manuell**: `kz.exe` aus dem [neuesten Release](https://github.com/hhasanguclu/kazma/releases/latest) herunterladen

> Die exe ist noch nicht code-signiert, daher kann SmartScreen warnen: **Weitere Informationen -> Trotzdem ausführen**.
> Die Signierung über SignPath ist vorbereitet, siehe [docs/SIGNING.md](docs/SIGNING.md).

**Deinstallation**: `kz remove` (entfernt die exe, ihren PATH-Eintrag und `%LOCALAPPDATA%\kazma`).

## Befehle

| Befehl | Funktion |
|---|---|
| `kz` | Interaktives Menü |
| `kz clean` | Temporäre Dateien, Absturzabbilder, Browser- / App- / Entwickler-Caches, Papierkorb |
| `kz deepclean` | Windows.old, Upgrade-Setupdateien, ESD-Dateien, Speicherabbilder, Ruhezustandsdatei, ältere Wiederherstellungspunkte (Admin) |
| `kz analyze [path]` | Explorer für die Datenträgerbelegung: `t` Treemap, `b` größte Dateien, `s` sortieren, `/` suchen, `w` wer nutzt es; erklärt spezielle Dateien (hiberfil, WSL-Datenträger, iPhone-Backups...); Gelöschtes landet im Papierkorb |
| `kz types [path]` | Welche Dateitypen den Platz belegen (Video, Bilder, Archive...) und die häufigsten Endungen |
| `kz purge [paths...]` | Build-Artefakte: `node_modules`, `target`, `bin/obj`, `.venv`, `__pycache__`, ... |
| `kz installer [paths...]` | Alte Setupdateien (`.exe`, `.msi`, `.msix`, `.iso`) in Downloads und auf dem Desktop |
| `kz dupes [paths...]` | Doppelte Dateien (Größe -> Teil-Hash -> BLAKE3, alles parallel) |
| `kz empty [paths...]` | Leere Ordner, defekte Verknüpfungen und 0-Byte-Dateien (in den Papierkorb) |
| `kz privacy` | Zuletzt verwendete Dateien, Ausführen- / Explorer-Verlauf, Zwischenablage, Browserverlauf und Cookies (Opt-in) |
| `kz uninstall` | Klassische, Microsoft Store- und Scoop-Apps entfernen, danach übrige Ordner und Registrierungsschlüssel (vorher gesichert); `--leftovers` findet Reste von Apps, die anders entfernt wurden |
| `kz upgrade` | Installierte Programme über winget aktualisieren |
| `kz drivers` | Treiberupdates über Windows Update; `--installed` listet Drittanbieter-Treiber nach Alter |
| `kz startup` | Autostart-Apps ein-/ausschalten (dieselben Schalter wie im Task-Manager, vollständig umkehrbar) |
| `kz debloat` | Vorinstallierte Werbe-Apps aus einer festen Liste entfernen, Werbung abschalten; `--restore` macht es rückgängig |
| `kz backup [folder]` | Umzugspaket für den neuen PC: winget-App-Liste, Treiber, Programmliste, kazma-Einstellungen; `kz backup restore FOLDER` installiert neu |
| `kz health` | SMART der Datenträger, Akkuverschleiß, Bluescreens, Fehler, Updates, Defender, Firewall |
| `kz status` | Live-Werte für CPU, GPU, Speicher, Datenträger, Netzwerk, Akku; `enter` erklärt, warum ein Prozess läuft, `x` beendet ihn, `m` sortiert nach Speicher |
| `kz power` | Energiesparpläne (`set saver / balanced / high / ultimate`), `battery`-Bericht, `wake`-Quellen, `hibernate off / reduced / full` |
| `kz restore` | Wiederherstellungspunkte der Systemwiederherstellung: `list`, `create "desc"`, `enable`, `open` (Windows-eigener Assistent) |
| `kz report` | HTML-Systembericht: Hardware, Zustand, Autostart, Programme, Treiber |
| `kz bench [path]` | Datenträger-Geschwindigkeitstest: sequenziell und 4K zufällig, ohne OS-Cache |
| `kz net` | IP und Latenz; `dns`-Benchmark, `dns set cloudflare`; `speed`-Test; `wifi [--show]` |
| `kz ports [filter]` | Welches Programm an welchem Port lauscht; `--kill PORT` schließt es |
| `kz optimize` | Defekte Autostarteinträge (gesichert), Start/Suche neu starten, DNS leeren, Symbolcache, SSD-TRIM / DISM / SFC bei Bedarf |
| `kz repair [fix]` | `network`, `update` (Windows Update zurücksetzen), `store`, `dism` (+SFC), `time`, `printer`; vorher Wiederherstellungspunkt (Admin) |
| `kz schedule weekly` | `kz clean --yes` automatisch ausführen (`daily`, `weekly`, `monthly`, `off`, `status`) |
| `kz lock <path>` | Welches Programm eine Datei oder einen Ordner geöffnet hat |
| `kz awake [minutes]` | Bildschirm und PC wach halten |
| `kz cleanscreen [seconds]` | Schwarzer Bildschirm, der Eingaben ignoriert, zum Abwischen des Displays |
| `kz whitelist` | Pfade, die kazma nie löschen darf (`--add`, `--remove`, `--edit`) |
| `kz rules` | Eigene Bereinigungsregeln für `kz clean` (`--edit` öffnet den Editor) |
| `kz context install` | „Analyze with kazma“ im Rechtsklickmenü des Explorers (`uninstall` entfernt es) |
| `kz config` | `kz config lang de` (auto, en, tr, zh-CN, zh-TW, ja, ko, de, fr, pl, uk, ru), `kz config theme amber` (green, amber, white, blue) |
| `kz update` | kazma selbst von GitHub aktualisieren (Prüfsumme verifiziert) |
| `kz history` | Protokoll aller Aktionen von kazma |
| `kz remove` | kazma deinstallieren |

Jeder Befehl, der löscht oder installiert, unterstützt `--dry-run` (`-n`) für eine Vorschau.
Aus einem Terminal **mit Adminrechten** (Als Administrator ausführen) starten, um Treiber zu installieren,
Wartungsaufgaben mit Adminrechten auszuführen und die Caches von Windows Update und System-Temp zu bereinigen.

## Sicherheit

- `clean` fasst nur bekannte, neu erzeugbare Cache- und Temp-Orte an; temporäre Dateien, die jünger als 24 h sind, bleiben erhalten.
- `purge` löscht einen Ordner nur, wenn daneben eine Projektdatei liegt (`package.json`, `Cargo.toml`, `.csproj`...).
- Laufwerkswurzeln, das Benutzerprofil, `Windows`, `Program Files`, `AppData` und ähnliche Ordner sind fest gesperrt.
- Reste aus `analyze`, `installer`, `dupes` und `uninstall` landen im **Papierkorb**.
- `dupes` behält immer mindestens eine Datei jeder Gruppe.
- `drivers` installiert nur Treiber, die **Windows Update** anbietet, und erstellt vorher einen **Wiederherstellungspunkt**.
- `startup` schaltet nur die Ein/Aus-Schalter des Task-Managers um; `debloat` entfernt nur Apps aus einer festen Liste, und `--restore` macht seine Einstellungen rückgängig.
- `update` prüft den Download gegen die `SHA256SUMS.txt` des Release.
- `deepclean` entfernt Windows.old und Upgradedateien über die Windows-eigene Datenträgerbereinigung und behält immer den neuesten Wiederherstellungspunkt.
- `repair` benennt die Windows Update-Ordner um, statt sie zu löschen, und bietet vorher einen Wiederherstellungspunkt an.
- Der MFT-Leser liest das Laufwerk nur; er schreibt nie darauf.
- Registrierungsreste und defekte Autostarteinträge werden vor dem Entfernen nach `%LOCALAPPDATA%\kazma\backups` exportiert.
- Vorschläge von `uninstall --leftovers` sind anfangs nicht ausgewählt und landen im Papierkorb.
- **Whitelist**: Pfade in `%LOCALAPPDATA%\kazma\whitelist.txt` werden nie gelöscht. Existiert die Datei, ist aber nicht lesbar, verweigert kazma den Start.
- Jede Aktion wird protokolliert (`kz history`); ohne Terminal wird nichts gelöscht, außer `--yes` ist angegeben.

## Build

```bash
cargo build --release                                  # on Windows -> target\release\kz.exe
rustup target add x86_64-pc-windows-gnu                # cross-compile from Linux
sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
```

Der Code lässt sich zur Entwicklung auch unter Linux/macOS bauen; Windows-spezifische Teile stehen hinter `cfg(windows)`.
Für ein Release `version` in `Cargo.toml` erhöhen und nach `main` pushen: Die CI testet unter Windows, führt jeden
schreibgeschützten Befehl real aus, veröffentlicht dann `vX.Y.Z` mit `kz.exe`, einem Zip und `SHA256SUMS.txt` und
aktualisiert die Scoop- / winget-Manifeste.

## Screenshots

| | |
|---|---|
| **Hauptmenü, Türkisch**<br><img src="docs/screenshots/menu-tr.png" alt="Hauptmenü, Türkisch"> | **Hauptmenü, Englisch, Bernstein-Theme**<br><img src="docs/screenshots/menu-en.png" alt="Hauptmenü, Englisch, Bernstein-Theme"> |
| **`kz analyze`**<br><img src="docs/screenshots/analyze-tr.png" alt="analyze"> | **`kz analyze` + `b`**: größte Dateien<br><img src="docs/screenshots/analyze-largest.png" alt="größte Dateien"> |
| **`kz analyze` + `t`**: Treemap<br><img src="docs/screenshots/treemap-tr.png" alt="Treemap"> | **`kz types`**: Dateitypen<br><img src="docs/screenshots/types-en.png" alt="Dateitypen"> |
| **`kz clean`** Auswahl<br><img src="docs/screenshots/clean-tr.png" alt="clean"> | **`kz purge`**, blaues Theme<br><img src="docs/screenshots/purge-en.png" alt="purge"> |
| **`kz health`** unter Windows<br><img src="docs/screenshots/health-windows.png" alt="Zustandsbericht unter Windows"> | **`kz startup --list`** unter Windows<br><img src="docs/screenshots/startup-windows.png" alt="Autostartliste unter Windows"> |

<sub>`health` und `startup` stammen aus einem echten Windows-Lauf (GitHub Actions, Windows Server 2025).
Die anderen Bildschirme stammen aus demselben Code, der in einem Terminal mit Demodaten lief; deshalb sehen
ihre Pfade wie `/home/user/demo` aus.</sub>

## Danksagung

**Inspiration.** kazma wurde von [Mole](https://github.com/tw93/Mole) von
[tw93](https://github.com/tw93) inspiriert, einem Tiefenreinigungs-Toolkit für macOS. kazma ist von Grund auf in Rust
geschrieben und teilt keinen Code damit.

**Übersetzungen.** Jede Sprache außer Englisch und Türkisch liegt in [`locales/`](locales) als JSON-Datei, die
den englischen Text der Übersetzung zuordnet. Korrekturen sind sehr willkommen: Datei bearbeiten und einen Pull Request öffnen
(`python tools/i18n_extract.py --missing de` listet, was einer Sprache noch fehlt).

**Bibliotheken.** Dank an die Autoren und Maintainer von:

| Crate | Verwendet für | Lizenz |
|---|---|---|
| [ratatui](https://github.com/ratatui/ratatui) | Terminal-UI | MIT |
| [crossterm](https://github.com/crossterm-rs/crossterm) | Terminaleingabe & Farben | MIT |
| [clap](https://github.com/clap-rs/clap) | Kommandozeile | MIT / Apache-2.0 |
| [sysinfo](https://github.com/GuillaumeGomez/sysinfo) | CPU, Speicher, Datenträger, Prozesse | MIT |
| [rayon](https://github.com/rayon-rs/rayon) | paralleles Scannen | MIT / Apache-2.0 |
| [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) | Hashing für Duplikate | CC0 / Apache-2.0 |
| [trash](https://github.com/Byron/trash-rs) | Papierkorb | MIT |
| [serde](https://github.com/serde-rs/serde), [serde_json](https://github.com/serde-rs/json) | PowerShell-JSON, Übersetzungen | MIT / Apache-2.0 |
| [anyhow](https://github.com/dtolnay/anyhow) | Fehler | MIT / Apache-2.0 |
| [syn](https://github.com/dtolnay/syn), [quote](https://github.com/dtolnay/quote) | das Übersetzungsmakro `tr!` | MIT / Apache-2.0 |
| [windows-sys](https://github.com/microsoft/windows-rs) | Windows API | MIT / Apache-2.0 |
| [winresource](https://github.com/BenjaminRi/winresource) | Icon & Versionsinfo der exe | MIT |

**Plattform.** Microsofts [winget](https://github.com/microsoft/winget-cli) (Programmupdates), die Windows Update
Agent API (Treiberupdates), [Scoop](https://scoop.sh) (Verteilung) und die
[SignPath Foundation](https://signpath.org) (kostenlose Code-Signierung für Open Source).

**Grafik.** Das Banner nutzt den klassischen [FIGlet](http://www.figlet.org)-Schriftstil „standard“; die Screenshots
sind mit der Schrift [DejaVu Sans Mono](https://dejavu-fonts.github.io) gerendert.

**Autor.** [hhasanguclu](https://github.com/hhasanguclu). Fehlerberichte, Ideen und Pull Requests sind willkommen.

## Lizenz

[MIT](LICENSE)
