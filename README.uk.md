<div align="center">

```
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
```

**Олдскульний набір системних утиліт для Windows, повністю в ASCII**

[![release](https://img.shields.io/github/v/release/hhasanguclu/kazma?style=flat-square)](https://github.com/hhasanguclu/kazma/releases/latest)
[![ci](https://img.shields.io/github/actions/workflow/status/hhasanguclu/kazma/ci.yml?branch=main&style=flat-square&label=windows%20ci)](https://github.com/hhasanguclu/kazma/actions)
[![license](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
![platform](https://img.shields.io/badge/windows-10%20%7C%2011-blue?style=flat-square)

[English](README.md) · [Türkçe](README.tr.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [Polski](README.pl.md) · **Українська** · [Русский](README.ru.md)

<img src="docs/screenshots/menu-tr.png" width="49%" alt="kazma main menu in Turkish"> <img src="docs/screenshots/menu-en.png" width="49%" alt="kazma main menu in English, amber theme">

</div>

---

**kazma** (турецькою «кайло») — це один файл `kz.exe` розміром ~2.7 MB, який очищає, досліджує й обслуговує
ПК з Windows просто з термінала, у ретростилі зеленого екрана. Без інсталятора, без фонових служб,
без телеметрії. Інтерфейс доступний 11 мовами (English, Türkçe, 简体中文, 繁體中文, 日本語, 한국어, Deutsch, Français, Polski,
Українська, Русский) і запускається мовою інтерфейсу Windows; змінити її можна командою `kz config lang de`.

## Можливості

| | |
|---|---|
| **Очищення** | понад 100 видів кешів, тимчасових файлів, журналів і Кошик, зокрема ШІ-інструменти, ігрові лаунчери та залишки драйверів відеокарти (`clean`) · Windows.old, файли оновлення системи, дампи пам’яті, файл гібернації, старі точки відновлення (`deepclean`) · провідник диска з мапою розмірів і списком найбільших файлів (`analyze`) · типи файлів (`types`) · артефакти збірки проєктів (`purge`) · старі інсталятори (`installer`) · дублікати файлів (`dupes`) · порожні папки й зламані ярлики (`empty`) · сліди використання (`privacy`) |
| **Програми й драйвери** | видалення програм та їхніх залишків (`uninstall`) · оновлення програм через winget (`upgrade`) · оновлення драйверів із Windows Update (`drivers`) · програми автозавантаження (`startup`) · рекламні програми й реклама Windows (`debloat`) · резервна копія для переїзду на новий ПК (`backup`) |
| **Система** | звіт про стан (`health`) · монітор у реальному часі з GPU й батареєю, диспетчер процесів, що пояснює, чому процес запущений (`status`) · обслуговування: DNS, кеш значків, SSD TRIM, DISM, SFC (`optimize`) · виправлення: мережа, Windows Update, Store, системні файли (`repair`) · схеми живлення й батарея (`power`) · точки відновлення (`restore`) · HTML-звіт (`report`) · тест швидкості диска (`bench`) · автоматичне очищення (`schedule`) |
| **Мережа** | IP, затримка, порівняння й перемикання DNS, тест швидкості, збережені мережі Wi-Fi (`net`) · яка програма використовує який порт (`ports`) |
| **Корисне** | яка програма тримає файл відкритим (`lock`) · не давати екрану згаснути (`awake`) · режим для протирання екрана (`cleanscreen`) |
| **kazma** | захищені шляхи (`whitelist`) · власні правила очищення (`rules`) · контекстне меню Провідника (`context`) · мова й тема (`config`) · самооновлення (`update`) · журнал (`history`) · видалення kazma (`remove`) |

У кожному списку є пошук через `/`, і всі вони працюють із мишею (клік — вибрати, ще клік — відкрити, коліщатко — прокрутка).
Меню повідомляє, коли виходить нова версія.

Вона швидка. Якщо запустити від імені адміністратора на диску NTFS, `analyze` і `types` читають головну таблицю файлів (MFT)
диска напряму (той самий трюк, що й у WizTree), тож розміри всього диска готові за лічені секунди. В інших випадках розміри
обчислюються паралельно, а `dupes` порівнює файли спершу за розміром, потім за першими й останніми
64 KB і лише тоді хешує файли повністю за допомогою BLAKE3 (83 000 файлів / 5.9 GB приблизно за 7 секунд).

## Встановлення

Windows 10/11, x64 і ARM64. Права адміністратора не потрібні.

```powershell
iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/install.ps1 | iex
```

Програма встановлюється в `%LOCALAPPDATA%\Programs\kazma` і додає `kz` до PATH. Інші способи:

- **Scoop**: `scoop bucket add kazma https://github.com/hhasanguclu/kazma`, потім `scoop install kazma`
- **winget**: маніфести для кожного випуску створюються в [`packaging/winget`](packaging/winget); після схвалення: `winget install hhasanguclu.kazma`
- **Вручну**: завантажте `kz.exe` з [останнього випуску](https://github.com/hhasanguclu/kazma/releases/latest)

> Exe-файл поки не має цифрового підпису, тому SmartScreen може показати попередження: **Докладніше -> Однаково запустити**.
> Підписування через SignPath уже підготовлено, див. [docs/SIGNING.md](docs/SIGNING.md).

**Видалення**: `kz remove` (видаляє exe-файл, його запис у PATH і `%LOCALAPPDATA%\kazma`).

## Команди

| Команда | Що робить |
|---|---|
| `kz` | Інтерактивне меню |
| `kz clean` | Тимчасові файли, аварійні дампи, кеші браузерів / програм / інструментів розробника, Кошик |
| `kz deepclean` | Windows.old, файли інсталяції оновлень, ESD-файли, дампи пам’яті, файл гібернації, старіші точки відновлення (адмін) |
| `kz analyze [path]` | Провідник використання диска: `t` мапа розмірів, `b` найбільші файли, `s` сортування, `/` пошук, `w` хто використовує; пояснює особливі файли (hiberfil, диски WSL, резервні копії iPhone...); видалене потрапляє в Кошик |
| `kz types [path]` | Які типи файлів займають місце (відео, зображення, архіви...) і найпоширеніші розширення |
| `kz purge [paths...]` | Артефакти збірки: `node_modules`, `target`, `bin/obj`, `.venv`, `__pycache__`, ... |
| `kz installer [paths...]` | Старі інсталятори (`.exe`, `.msi`, `.msix`, `.iso`) у Downloads і на Робочому столі |
| `kz dupes [paths...]` | Дублікати файлів (розмір -> частковий хеш -> BLAKE3, усе паралельно) |
| `kz empty [paths...]` | Порожні папки, зламані ярлики й файли 0 байтів (у Кошик) |
| `kz privacy` | Недавні файли, історія «Виконати» / Провідника, буфер обміну, історія й файли cookie браузерів (за вибором) |
| `kz uninstall` | Видаляє класичні програми, програми з Microsoft Store і Scoop, а потім залишки папок і розділів реєстру (спершу з резервною копією); `--leftovers` знаходить залишки програм, видалених іншим способом |
| `kz upgrade` | Оновлює встановлені програми через winget |
| `kz drivers` | Оновлення драйверів із Windows Update; `--installed` показує сторонні драйвери за віком |
| `kz startup` | Вмикає/вимикає програми автозавантаження (ті самі прапорці, що й у Диспетчері завдань, повністю оборотно) |
| `kz debloat` | Видаляє попередньо встановлені рекламні програми з фіксованого списку, вимикає рекламу; `--restore` скасовує |
| `kz backup [folder]` | Набір для нового ПК: список програм winget, драйвери, перелік програм, налаштування kazma; `kz backup restore FOLDER` встановлює все знову |
| `kz health` | SMART дисків, знос батареї, сині екрани, помилки, оновлення, Defender, брандмауер |
| `kz status` | CPU, GPU, пам’ять, диск, мережа, батарея в реальному часі; `enter` пояснює, чому процес запущений, `x` завершує його, `m` сортує за пам’яттю |
| `kz power` | Схеми живлення (`set saver / balanced / high / ultimate`), звіт `battery`, джерела пробудження `wake`, `hibernate off / reduced / full` |
| `kz restore` | Точки відновлення системи: `list`, `create "desc"`, `enable`, `open` (власний майстер Windows) |
| `kz report` | HTML-звіт про систему: обладнання, стан, автозавантаження, програми, драйвери |
| `kz bench [path]` | Тест швидкості диска: послідовні й випадкові 4K операції в обхід кешу ОС |
| `kz net` | IP і затримка; порівняння `dns`, `dns set cloudflare`; тест `speed`; `wifi [--show]` |
| `kz ports [filter]` | Яка програма прослуховує який порт; `--kill PORT` закриває її |
| `kz optimize` | Зламані записи автозавантаження (з резервною копією), перезапуск Пуску/Пошуку, очищення DNS, кеш значків, SSD TRIM / DISM / SFC за потреби |
| `kz repair [fix]` | `network`, `update` (скидання Windows Update), `store`, `dism` (+SFC), `time`, `printer`; спершу точка відновлення (адмін) |
| `kz schedule weekly` | Автоматично запускає `kz clean --yes` (`daily`, `weekly`, `monthly`, `off`, `status`) |
| `kz lock <path>` | Яка програма тримає відкритим файл або папку |
| `kz awake [minutes]` | Не дає екрану й ПК заснути |
| `kz cleanscreen [seconds]` | Чорний екран, що ігнорує введення, щоб протерти дисплей |
| `kz whitelist` | Шляхи, які kazma ніколи не видалить (`--add`, `--remove`, `--edit`) |
| `kz rules` | Власні правила очищення для `kz clean` (`--edit` відкриває Блокнот) |
| `kz context install` | «Analyze with kazma» у контекстному меню Провідника (`uninstall` прибирає) |
| `kz config` | `kz config lang de` (auto, en, tr, zh-CN, zh-TW, ja, ko, de, fr, pl, uk, ru), `kz config theme amber` (green, amber, white, blue) |
| `kz update` | Оновлює саму kazma з GitHub (з перевіркою контрольної суми) |
| `kz history` | Журнал усього, що зробила kazma |
| `kz remove` | Видаляє kazma |

Кожна команда, що видаляє або встановлює, підтримує `--dry-run` (`-n`) для попереднього перегляду.
Запускайте з терміналу **з правами адміністратора** (Запуск від імені адміністратора), щоб встановлювати драйвери, виконувати
адміністративні завдання обслуговування й очищати кеші Windows Update і системні тимчасові файли.

## Безпека

- `clean` торкається лише відомих місць кешу й тимчасових файлів, які відтворюються самі; тимчасові файли, молодші за 24 год, залишаються.
- `purge` видаляє папку, лише якщо поруч лежить маркер проєкту (`package.json`, `Cargo.toml`, `.csproj`...).
- Корені дисків, ваш профіль, `Windows`, `Program Files`, `AppData` та подібні папки жорстко заблоковано.
- Видалене в `analyze`, `installer`, `dupes` і залишки `uninstall` потрапляють у **Кошик**.
- `dupes` завжди залишає щонайменше один файл із кожної групи.
- `drivers` встановлює лише драйвери, які пропонує **Windows Update**, і спершу створює **точку відновлення**.
- `startup` лише перемикає прапорці увімк./вимк. Диспетчера завдань; `debloat` видаляє лише програми з фіксованого списку, а `--restore` скасовує його налаштування.
- `update` перевіряє завантажений файл за `SHA256SUMS.txt` випуску.
- `deepclean` видаляє Windows.old і файли оновлення через власне Очищення диска Windows і завжди залишає найновішу точку відновлення.
- `repair` перейменовує папки Windows Update замість видалення й спершу пропонує створити точку відновлення.
- Зчитувач MFT лише читає диск і ніколи нічого на нього не записує.
- Залишки в реєстрі та зламані записи автозавантаження перед видаленням експортуються в `%LOCALAPPDATA%\kazma\backups`.
- Пропозиції `uninstall --leftovers` спочатку не вибрані й потрапляють у Кошик.
- **Білий список**: шляхи з `%LOCALAPPDATA%\kazma\whitelist.txt` ніколи не видаляються. Якщо файл існує, але його не вдається прочитати, kazma відмовляється працювати.
- Кожна дія записується в журнал (`kz history`); без термінала нічого не видаляється, якщо не передано `--yes`.

## Збирання

```bash
cargo build --release                                  # on Windows -> target\release\kz.exe
rustup target add x86_64-pc-windows-gnu                # cross-compile from Linux
sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
```

Для розробки код збирається також на Linux/macOS; частини лише для Windows сховано за `cfg(windows)`.
Щоб випустити реліз, збільште `version` у `Cargo.toml` і надішліть зміни в `main`: CI тестує на Windows, реально запускає кожну
команду, що лише читає дані, потім публікує `vX.Y.Z` з `kz.exe`, zip-архівом і `SHA256SUMS.txt` та оновлює
маніфести Scoop / winget.

## Знімки екрана

| | |
|---|---|
| **Головне меню, турецькою**<br><img src="docs/screenshots/menu-tr.png" alt="main menu, Turkish"> | **Головне меню, англійською, тема amber**<br><img src="docs/screenshots/menu-en.png" alt="main menu, English, amber theme"> |
| **`kz analyze`**<br><img src="docs/screenshots/analyze-tr.png" alt="analyze"> | **`kz analyze` + `b`**: найбільші файли<br><img src="docs/screenshots/analyze-largest.png" alt="largest files"> |
| **`kz analyze` + `t`**: мапа розмірів<br><img src="docs/screenshots/treemap-tr.png" alt="treemap"> | **`kz types`**: типи файлів<br><img src="docs/screenshots/types-en.png" alt="file types"> |
| Вибір у **`kz clean`**<br><img src="docs/screenshots/clean-tr.png" alt="clean"> | **`kz purge`**, тема blue<br><img src="docs/screenshots/purge-en.png" alt="purge"> |
| **`kz health`** у Windows<br><img src="docs/screenshots/health-windows.png" alt="health report on Windows"> | **`kz startup --list`** у Windows<br><img src="docs/screenshots/startup-windows.png" alt="startup list on Windows"> |

<sub>`health` і `startup` знято під час реального запуску у Windows (GitHub Actions, Windows Server 2025).
Інші екрани отримано з того самого коду, запущеного в терміналі з демонстраційними даними, тому шляхи на них
мають вигляд `/home/user/demo`.</sub>

## Подяки

**Натхнення.** kazma створено під враженням від [Mole](https://github.com/tw93/Mole) автора
[tw93](https://github.com/tw93) — набору для глибокого очищення macOS. kazma написана з нуля на Rust і
не має з ним спільного коду.

**Переклади.** Кожна мова, крім англійської й турецької, зберігається в [`locales/`](locales) як JSON-файл,
що зіставляє англійський текст із перекладом. Виправлення дуже вітаються: відредагуйте файл і відкрийте pull request
(`python tools/i18n_extract.py --missing de` показує, чого ще бракує мові).

**Бібліотеки.** Дякуємо авторам і супровідникам:

| Crate | Для чого | Ліцензія |
|---|---|---|
| [ratatui](https://github.com/ratatui/ratatui) | інтерфейс термінала | MIT |
| [crossterm](https://github.com/crossterm-rs/crossterm) | введення й кольори в терміналі | MIT |
| [clap](https://github.com/clap-rs/clap) | командний рядок | MIT / Apache-2.0 |
| [sysinfo](https://github.com/GuillaumeGomez/sysinfo) | CPU, пам’ять, диски, процеси | MIT |
| [rayon](https://github.com/rayon-rs/rayon) | паралельне сканування | MIT / Apache-2.0 |
| [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) | хешування дублікатів | CC0 / Apache-2.0 |
| [trash](https://github.com/Byron/trash-rs) | Кошик | MIT |
| [serde](https://github.com/serde-rs/serde), [serde_json](https://github.com/serde-rs/json) | JSON із PowerShell, переклади | MIT / Apache-2.0 |
| [anyhow](https://github.com/dtolnay/anyhow) | помилки | MIT / Apache-2.0 |
| [syn](https://github.com/dtolnay/syn), [quote](https://github.com/dtolnay/quote) | макрос перекладу `tr!` | MIT / Apache-2.0 |
| [windows-sys](https://github.com/microsoft/windows-rs) | Windows API | MIT / Apache-2.0 |
| [winresource](https://github.com/BenjaminRi/winresource) | значок і відомості про версію exe | MIT |

**Платформа.** [winget](https://github.com/microsoft/winget-cli) від Microsoft (оновлення програм), Windows Update
Agent API (оновлення драйверів), [Scoop](https://scoop.sh) (поширення) і
[SignPath Foundation](https://signpath.org) (безкоштовне підписування коду для open source).

**Оформлення.** Банер використовує класичний шрифт «standard» з [FIGlet](http://www.figlet.org); знімки екрана
відрендерено шрифтом [DejaVu Sans Mono](https://dejavu-fonts.github.io).

**Автор.** [hhasanguclu](https://github.com/hhasanguclu). Повідомлення про помилки, ідеї та pull request-и вітаються.

## Ліцензія

[MIT](LICENSE)
