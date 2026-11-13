<div align="center">

```
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
```

**Oldschoolowy zestaw narzędzi systemowych dla Windows, w całości w ASCII**

[![release](https://img.shields.io/github/v/release/hhasanguclu/kazma?style=flat-square)](https://github.com/hhasanguclu/kazma/releases/latest)
[![ci](https://img.shields.io/github/actions/workflow/status/hhasanguclu/kazma/ci.yml?branch=main&style=flat-square&label=windows%20ci)](https://github.com/hhasanguclu/kazma/actions)
[![license](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
![platform](https://img.shields.io/badge/windows-10%20%7C%2011-blue?style=flat-square)

[English](README.md) · [Türkçe](README.tr.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · **Polski** · [Українська](README.uk.md) · [Русский](README.ru.md)

<img src="docs/screenshots/menu-en.png" width="49%" alt="kazma main menu in English"> <img src="docs/screenshots/menu-ja.png" width="49%" alt="kazma main menu in Japanese, blue theme">

</div>

---

**kazma** (po turecku *kilof*) to pojedynczy plik `kz.exe` o rozmiarze ok. 2,7 MB, który czyści, przegląda i konserwuje
komputer z Windows z poziomu terminala, w stylu retro zielonego ekranu. Bez instalatora, bez usługi w tle,
bez telemetrii. Interfejs jest dostępny w 11 językach (English, Türkçe, 简体中文, 繁體中文, 日本語, 한국어, Deutsch, Français, Polski,
Українська, Русский) i uruchamia się w języku wyświetlania Windows; zmienisz go poleceniem `kz config lang pl`.

## Funkcje

| | |
|---|---|
| **Sprzątanie** | ponad 100 rodzajów pamięci podręcznej, plików tymczasowych, logów oraz Kosz, w tym narzędzia AI, launchery gier i pozostałości sterowników GPU (`clean`) · Windows.old, pliki aktualizacji, zrzuty pamięci, plik hibernacji, stare punkty przywracania (`deepclean`) · eksplorator dysku z mapą drzewa i widokiem największych plików (`analyze`) · typy plików (`types`) · artefakty kompilacji projektów (`purge`) · stare instalatory (`installer`) · zduplikowane pliki (`dupes`) · puste foldery i uszkodzone skróty (`empty`) · ślady użytkowania (`privacy`) |
| **Programy i sterowniki** | odinstalowywanie programów wraz z pozostałościami (`uninstall`) · aktualizacja programów przez winget (`upgrade`) · aktualizacje sterowników z Windows Update (`drivers`) · aplikacje autostartu (`startup`) · aplikacje promocyjne i reklamy Windows (`debloat`) · kopia do przeprowadzki na nowy PC (`backup`) |
| **System** | raport kondycji (`health`) · monitor na żywo z GPU i baterią oraz menedżer procesów, który wyjaśnia, dlaczego proces działa (`status`) · konserwacja: DNS, cache ikon, SSD TRIM, DISM, SFC (`optimize`) · naprawy: sieć, Windows Update, Store, pliki systemowe (`repair`) · plany zasilania i bateria (`power`) · punkty przywracania (`restore`) · raport HTML (`report`) · test szybkości dysku (`bench`) · automatyczne czyszczenie (`schedule`) |
| **Sieć** | IP, opóźnienie, test i zmiana DNS, test prędkości, zapisane sieci Wi-Fi (`net`) · kto używa którego portu (`ports`) |
| **Przydatne** | który program trzyma otwarty plik (`lock`) · nie wygaszaj ekranu (`awake`) · tryb do przecierania ekranu (`cleanscreen`) |
| **kazma** | chronione ścieżki (`whitelist`) · własne reguły czyszczenia (`rules`) · menu prawego przycisku w Eksploratorze plików (`context`) · język i motyw (`config`) · samoaktualizacja (`update`) · dziennik (`history`) · odinstalowanie kazma (`remove`) |

Każda lista ma wyszukiwanie `/` i obsługuje mysz (kliknij, aby zaznaczyć, kliknij ponownie, aby otworzyć, kółko przewija).
Menu informuje, gdy pojawi się nowa wersja.

Jest szybka. Uruchomione jako administrator na dysku NTFS, `analyze` i `types` odczytują bezpośrednio Master File Table
dysku (sztuczka, na której opiera się WizTree), więc rozmiary całego dysku są gotowe w kilka sekund. W pozostałych przypadkach rozmiary
są liczone równolegle, a `dupes` porównuje najpierw rozmiar, potem pierwsze i ostatnie
64 KB, a dopiero na końcu haszuje całe pliki przez BLAKE3 (83 000 plików / 5,9 GB w około 7 sekund).

## Instalacja

Windows 10/11, x64 i ARM64. Uprawnienia administratora nie są potrzebne.

```powershell
iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/install.ps1 | iex
```

Instaluje do `%LOCALAPPDATA%\Programs\kazma` i dodaje `kz` do PATH. Alternatywy:

- **Scoop**: `scoop bucket add kazma https://github.com/hhasanguclu/kazma`, a potem `scoop install kazma`
- **winget**: manifesty każdego wydania są generowane w [`packaging/winget`](packaging/winget); po ich zaakceptowaniu: `winget install hhasanguclu.kazma`
- **Ręcznie**: pobierz `kz.exe` z [najnowszego wydania](https://github.com/hhasanguclu/kazma/releases/latest)

> Plik exe nie jest jeszcze podpisany cyfrowo, więc SmartScreen może ostrzec: **Więcej informacji -> Uruchom mimo to**.
> Podpisywanie przez SignPath jest przygotowane, zobacz [docs/SIGNING.md](docs/SIGNING.md).

**Odinstalowanie**: `kz remove` (usuwa exe, jego wpis w PATH oraz `%LOCALAPPDATA%\kazma`).

## Polecenia

| Polecenie | Co robi |
|---|---|
| `kz` | Menu interaktywne |
| `kz clean` | Pliki tymczasowe, zrzuty awarii, pamięć podręczna przeglądarek / aplikacji / narzędzi programistycznych, Kosz |
| `kz deepclean` | Windows.old, pliki instalacyjne aktualizacji, pliki ESD, zrzuty pamięci, plik hibernacji, starsze punkty przywracania (admin) |
| `kz analyze [path]` | Eksplorator zajętości dysku: `t` mapa drzewa, `b` największe pliki, `s` sortowanie, `/` wyszukiwanie, `w` kto używa; objaśnia pliki specjalne (hiberfil, dyski WSL, kopie iPhone'a...); usuwanie trafia do Kosza |
| `kz types [path]` | Jakie pliki zajmują miejsce (wideo, obrazy, archiwa...) i najczęstsze rozszerzenia |
| `kz purge [paths...]` | Artefakty kompilacji: `node_modules`, `target`, `bin/obj`, `.venv`, `__pycache__`, ... |
| `kz installer [paths...]` | Stare pliki instalacyjne (`.exe`, `.msi`, `.msix`, `.iso`) w Pobranych i na Pulpicie |
| `kz dupes [paths...]` | Zduplikowane pliki (rozmiar -> częściowy hash -> BLAKE3, wszystko równolegle) |
| `kz empty [paths...]` | Puste foldery, uszkodzone skróty i pliki 0-bajtowe (do Kosza) |
| `kz privacy` | Ostatnie pliki, historia Uruchamiania / Eksploratora, schowek, historia i pliki cookie przeglądarek (do wyboru) |
| `kz uninstall` | Usuwa aplikacje klasyczne, z Microsoft Store i ze Scoop, a potem pozostałe foldery i klucze rejestru (najpierw z kopią zapasową); `--leftovers` znajduje resztki aplikacji usuniętych w inny sposób |
| `kz upgrade` | Aktualizuje zainstalowane programy przez winget |
| `kz drivers` | Aktualizacje sterowników z Windows Update; `--installed` wyświetla sterowniki innych firm według wieku |
| `kz startup` | Włącza/wyłącza aplikacje autostartu (te same flagi co Menedżer zadań, w pełni odwracalne) |
| `kz debloat` | Usuwa preinstalowane aplikacje promocyjne ze stałej listy, wyłącza reklamy; `--restore` cofa zmiany |
| `kz backup [folder]` | Zestaw na nowy PC: lista aplikacji winget, sterowniki, lista programów, ustawienia kazma; `kz backup restore FOLDER` instaluje ponownie |
| `kz health` | SMART dysków, zużycie baterii, niebieskie ekrany, błędy, aktualizacje, Defender, zapora |
| `kz status` | Na żywo CPU, GPU, pamięć, dysk, sieć, bateria; `enter` wyjaśnia, dlaczego proces działa, `x` go kończy, `m` sortuje według pamięci |
| `kz power` | Plany zasilania (`set saver / balanced / high / ultimate`), raport `battery`, źródła `wake`, `hibernate off / reduced / full` |
| `kz restore` | Punkty Przywracania systemu: `list`, `create "desc"`, `enable`, `open` (kreator samego Windows) |
| `kz report` | Raport systemu w HTML: sprzęt, kondycja, autostart, programy, sterowniki |
| `kz bench [path]` | Test szybkości dysku: sekwencyjny i losowy 4K, z pominięciem pamięci podręcznej systemu |
| `kz net` | IP i opóźnienie; test `dns`, `dns set cloudflare`; test `speed`; `wifi [--show]` |
| `kz ports [filter]` | Który program nasłuchuje na którym porcie; `--kill PORT` go zamyka |
| `kz optimize` | Uszkodzone wpisy autostartu (z kopią zapasową), restart Start/Wyszukiwania, opróżnienie DNS, cache ikon, SSD TRIM / DISM / SFC, gdy trzeba |
| `kz repair [fix]` | `network`, `update` (reset Windows Update), `store`, `dism` (+SFC), `time`, `printer`; najpierw punkt przywracania (admin) |
| `kz schedule weekly` | Automatycznie uruchamia `kz clean --yes` (`daily`, `weekly`, `monthly`, `off`, `status`) |
| `kz lock <path>` | Który program ma otwarty plik lub folder |
| `kz awake [minutes]` | Nie pozwala wygasić ekranu ani uśpić komputera |
| `kz cleanscreen [seconds]` | Czarny ekran ignorujący wejście, do przetarcia monitora |
| `kz whitelist` | Ścieżki, których kazma nigdy nie usunie (`--add`, `--remove`, `--edit`) |
| `kz rules` | Własne reguły czyszczenia dla `kz clean` (`--edit` otwiera Notatnik) |
| `kz context install` | „Analyze with kazma” w menu prawego przycisku Eksploratora plików (`uninstall` usuwa wpis) |
| `kz config` | `kz config` ↑↓ enter · `kz config lang de` (auto, en, tr, zh-CN, zh-TW, ja, ko, de, fr, pl, uk, ru), `kz config theme amber` (green, amber, white, blue) |
| `kz update` | Aktualizuje kazma z GitHuba (z weryfikacją sumy kontrolnej) |
| `kz history` | Dziennik wszystkiego, co zrobiła kazma |
| `kz remove` | Odinstalowuje kazma |

Każde polecenie, które usuwa lub instaluje, obsługuje `--dry-run` (`-n`), aby najpierw zobaczyć podgląd.
Uruchom w terminalu **z uprawnieniami administratora** (Uruchom jako administrator), aby instalować sterowniki, wykonywać
zadania konserwacyjne wymagające administratora oraz czyścić pamięć podręczną Windows Update i systemowe pliki tymczasowe.

## Bezpieczeństwo

- `clean` rusza tylko znane, odtwarzalne lokalizacje pamięci podręcznej i plików tymczasowych; pliki tymczasowe młodsze niż 24 h zostają.
- `purge` usuwa folder tylko wtedy, gdy obok leży znacznik projektu (`package.json`, `Cargo.toml`, `.csproj`...).
- Katalogi główne dysków, Twój profil, `Windows`, `Program Files`, `AppData` i podobne foldery są bezwzględnie zablokowane.
- Pliki z `analyze`, `installer`, `dupes` i pozostałości z `uninstall` trafiają do **Kosza**.
- `dupes` zawsze zostawia co najmniej jeden plik z każdej grupy.
- `drivers` instaluje tylko sterowniki oferowane przez **Windows Update** i najpierw tworzy **punkt przywracania**.
- `startup` przełącza jedynie flagi wł./wył. Menedżera zadań; `debloat` usuwa tylko aplikacje ze stałej listy, a `--restore` cofa jego ustawienia.
- `update` weryfikuje pobrany plik względem `SHA256SUMS.txt` z wydania.
- `deepclean` usuwa Windows.old i pliki aktualizacji za pomocą Oczyszczania dysku samego Windows i zawsze zostawia najnowszy punkt przywracania.
- `repair` zmienia nazwy folderów Windows Update zamiast je usuwać i najpierw proponuje punkt przywracania.
- Czytnik MFT tylko odczytuje dysk; nigdy na nim nie zapisuje.
- Pozostałości w rejestrze i uszkodzone wpisy autostartu są eksportowane do `%LOCALAPPDATA%\kazma\backups` przed usunięciem.
- Propozycje `uninstall --leftovers` są domyślnie niezaznaczone i trafiają do Kosza.
- **Biała lista**: ścieżki z `%LOCALAPPDATA%\kazma\whitelist.txt` nigdy nie są usuwane. Jeśli ten plik istnieje, ale nie da się go odczytać, kazma odmawia działania.
- Każda operacja jest zapisywana w dzienniku (`kz history`); bez terminala nic nie zostanie usunięte, chyba że podano `--yes`.

## Kompilacja

```bash
cargo build --release                                  # on Windows -> target\release\kz.exe
rustup target add x86_64-pc-windows-gnu                # cross-compile from Linux
sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
```

Kod kompiluje się też na Linuksie/macOS na potrzeby programowania; części tylko dla Windows są za `cfg(windows)`.
Aby wydać nową wersję, podnieś `version` w `Cargo.toml` i wypchnij do `main`: CI testuje na Windows, uruchamia naprawdę każde
polecenie tylko do odczytu, a potem publikuje `vX.Y.Z` z `kz.exe`, archiwum zip i `SHA256SUMS.txt` oraz aktualizuje
manifesty Scoop / winget.

## Zrzuty ekranu

| | |
|---|---|
| **`kz`** · English<br><img src="docs/screenshots/menu-en.png" alt="kz main menu, English"> | **`kz`** · 日本語<br><img src="docs/screenshots/menu-ja.png" alt="kz main menu, Japanese, blue theme"> |
| **`kz`** · Türkçe<br><img src="docs/screenshots/menu-tr.png" alt="kz main menu, Turkish, amber theme"> | **`kz analyze`** · `(i)`<br><img src="docs/screenshots/analyze-tr.png" alt="kz analyze"> |
| **`kz analyze`** + `b`<br><img src="docs/screenshots/analyze-largest.png" alt="kz analyze, largest files"> | **`kz analyze`** + `t`<br><img src="docs/screenshots/treemap-tr.png" alt="kz analyze, treemap"> |
| **`kz clean`** · Türkçe<br><img src="docs/screenshots/clean-tr.png" alt="kz clean, Turkish"> | **`kz clean`** · Deutsch<br><img src="docs/screenshots/clean-de.png" alt="kz clean, German, white theme"> |
| **`kz types`**<br><img src="docs/screenshots/types-en.png" alt="kz types"> | **`kz purge`**<br><img src="docs/screenshots/purge-en.png" alt="kz purge, blue theme"> |
| **`kz health`** · Windows<br><img src="docs/screenshots/health-windows.png" alt="kz health on Windows"> | **`kz health`** · Windows · 日本語<br><img src="docs/screenshots/health-windows-ja.png" alt="kz health on Windows, Japanese"> |
| **`kz startup --list`** · Windows<br><img src="docs/screenshots/startup-windows.png" alt="kz startup on Windows"> |  |

<sub>`health` i `startup` zostały przechwycone z prawdziwego uruchomienia w Windows (GitHub Actions, Windows Server 2025).
Pozostałe ekrany pochodzą z tego samego kodu uruchomionego w terminalu z danymi demonstracyjnymi, dlatego ich ścieżki
wyglądają jak `/home/user/demo`.</sub>

## Podziękowania

**Inspiracja.** kazma powstała z inspiracji projektem [Mole](https://github.com/tw93/Mole) autorstwa
[tw93](https://github.com/tw93), zestawem do gruntownego czyszczenia macOS. kazma została napisana od zera w Rust i
nie dzieli z nim kodu.

**Tłumaczenia.** Każdy język poza angielskim i tureckim znajduje się w [`locales/`](locales) jako plik JSON,
który przypisuje tekstowi angielskiemu tłumaczenie. Poprawki są bardzo mile widziane: edytuj plik i otwórz pull request
(`python tools/i18n_extract.py --missing de` pokazuje, czego jeszcze brakuje w danym języku).

**Biblioteki.** Podziękowania dla autorów i opiekunów:

| Crate | Zastosowanie | Licencja |
|---|---|---|
| [ratatui](https://github.com/ratatui/ratatui) | interfejs terminalowy | MIT |
| [crossterm](https://github.com/crossterm-rs/crossterm) | wejście i kolory terminala | MIT |
| [clap](https://github.com/clap-rs/clap) | wiersz poleceń | MIT / Apache-2.0 |
| [sysinfo](https://github.com/GuillaumeGomez/sysinfo) | CPU, pamięć, dyski, procesy | MIT |
| [rayon](https://github.com/rayon-rs/rayon) | równoległe skanowanie | MIT / Apache-2.0 |
| [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) | haszowanie duplikatów | CC0 / Apache-2.0 |
| [trash](https://github.com/Byron/trash-rs) | Kosz | MIT |
| [serde](https://github.com/serde-rs/serde), [serde_json](https://github.com/serde-rs/json) | JSON z PowerShell, tłumaczenia | MIT / Apache-2.0 |
| [anyhow](https://github.com/dtolnay/anyhow) | błędy | MIT / Apache-2.0 |
| [syn](https://github.com/dtolnay/syn), [quote](https://github.com/dtolnay/quote) | makro tłumaczeń `tr!` | MIT / Apache-2.0 |
| [windows-sys](https://github.com/microsoft/windows-rs) | Windows API | MIT / Apache-2.0 |
| [winresource](https://github.com/BenjaminRi/winresource) | ikona exe i informacje o wersji | MIT |

**Platforma.** [winget](https://github.com/microsoft/winget-cli) od Microsoftu (aktualizacje programów), Windows Update
Agent API (aktualizacje sterowników), [Scoop](https://scoop.sh) (dystrybucja) oraz
[SignPath Foundation](https://signpath.org) (darmowe podpisywanie kodu dla open source).

**Grafika.** Baner używa klasycznego stylu liter „standard” z [FIGlet](http://www.figlet.org); zrzuty ekranu
są renderowane czcionką [DejaVu Sans Mono](https://dejavu-fonts.github.io).

**Autor.** [hhasanguclu](https://github.com/hhasanguclu). Zgłoszenia błędów, pomysły i pull requesty są mile widziane.

## Licencja

[MIT](LICENSE)
