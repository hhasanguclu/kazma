<div align="center">

```
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
```

**Oldschool, all-ASCII system toolkit for Windows**<br>
**Windows için oldschool, tamamen ASCII sistem araç kutusu**

[![release](https://img.shields.io/github/v/release/hhasanguclu/kazma?style=flat-square)](https://github.com/hhasanguclu/kazma/releases/latest)
[![ci](https://img.shields.io/github/actions/workflow/status/hhasanguclu/kazma/ci.yml?branch=main&style=flat-square&label=windows%20ci)](https://github.com/hhasanguclu/kazma/actions)
[![license](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
![platform](https://img.shields.io/badge/windows-10%20%7C%2011-blue?style=flat-square)

[English](#english) · [Türkçe](#türkçe) · [Screenshots / Ekran görüntüleri](#screenshots--ekran-görüntüleri) · [Credits / Teşekkürler](#credits--teşekkürler)

<img src="docs/screenshots/menu-tr.png" width="49%" alt="kazma main menu in Turkish"> <img src="docs/screenshots/menu-en.png" width="49%" alt="kazma main menu in English, amber theme">

</div>

---

## English

**kazma** (Turkish for *pickaxe*) is a single ~2.7 MB `kz.exe` that cleans, explores and maintains a
Windows PC from the terminal, with a retro green-screen look. No installer, no background service,
no telemetry. The interface speaks English and Turkish and follows your Windows display language.

### Features

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

### Install

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

### Commands

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
| `kz config` | `kz config lang tr`, `kz config theme amber` (green, amber, white, blue) |
| `kz update` | Update kazma itself from GitHub (checksum verified) |
| `kz history` | Log of everything kazma did |
| `kz remove` | Uninstall kazma |

Every command that deletes or installs supports `--dry-run` (`-n`) to preview first.
Run from an **elevated** terminal (Run as administrator) to install drivers, run admin maintenance
tasks, and clean Windows Update and system temp caches.

### Safety

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

### Build

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

---

## Türkçe

**kazma**, Windows bilgisayarı terminalden temizleyen, inceleyen ve bakımını yapan tek bir
`kz.exe` dosyasıdır (~2.7 MB). Görünümü eski yeşil ekranlı terminaller gibidir. Kurulum gerektirmez,
arka planda servis çalıştırmaz, veri toplamaz. Arayüz Türkçe ve İngilizcedir; Windows diliniz
Türkçeyse kendiliğinden Türkçe açılır.

### Özellikler

| | |
|---|---|
| **Temizlik** | yapay zekâ araçları, oyun başlatıcıları ve ekran kartı sürücü artıkları dahil 100+ tür önbellek, geçici dosya, log ve Geri Dönüşüm Kutusu (`clean`) · Windows.old, yükseltme dosyaları, bellek dökümleri, hazırda bekletme dosyası, eski geri yükleme noktaları (`deepclean`) · ağaç haritalı ve en büyük dosyalar görünümlü disk gezgini (`analyze`) · dosya türleri (`types`) · proje derleme çıktıları (`purge`) · eski kurulum dosyaları (`installer`) · yinelenen dosyalar (`dupes`) · boş klasörler ve bozuk kısayollar (`empty`) · kullanım izleri (`privacy`) |
| **Programlar ve sürücüler** | program ve kalıntılarını kaldırma (`uninstall`) · winget ile program güncelleme (`upgrade`) · Windows Update'ten sürücü güncelleme (`drivers`) · başlangıç programları (`startup`) · tanıtım uygulamaları ve Windows reklamları (`debloat`) · yeni bilgisayara taşıma yedeği (`backup`) |
| **Sistem** | sağlık raporu (`health`) · GPU ve pil dahil canlı izleme, bir sürecin neden çalıştığını açıklayan süreç yöneticisi (`status`) · bakım: DNS, ikon önbelleği, SSD TRIM, DISM, SFC (`optimize`) · onarım: ağ, Windows Update, Store, sistem dosyaları (`repair`) · güç planı ve pil (`power`) · geri yükleme noktaları (`restore`) · HTML rapor (`report`) · disk hız testi (`bench`) · otomatik temizlik (`schedule`) |
| **Ağ** | IP, gecikme, DNS karşılaştırma ve değiştirme, hız testi, kayıtlı Wi-Fi (`net`) · hangi port hangi programda (`ports`) |
| **Pratik** | dosyayı hangi programın açık tuttuğu (`lock`) · ekranı açık tutma (`awake`) · ekran silme modu (`cleanscreen`) |
| **kazma** | korunan yollar (`whitelist`) · kendi temizlik kuralların (`rules`) · Gezgin sağ tık menüsü (`context`) · dil ve tema (`config`) · kendini güncelleme (`update`) · geçmiş (`history`) · kazma'yı kaldırma (`remove`) |

Her listede `/` ile arama var ve fare çalışır (tıkla seç, tekrar tıkla aç, tekerlekle kaydır).
Yeni sürüm çıkınca menü haber verir.

Hızlıdır. NTFS bir diskte yönetici olarak çalıştırıldığında `analyze` ve `types` diskin Ana Dosya Tablosu'nu (MFT)
doğrudan okur (WizTree'nin hızının sırrı); tüm diskin boyutları saniyeler içinde hazır olur. Aksi halde boyutlar
paralel hesaplanır. `dupes` önce boyuta, sonra dosyanın ilk ve son 64 KB'ına
bakar; yalnızca hâlâ eşleşenlerin tamamını BLAKE3 ile karşılaştırır (83.000 dosya / 5,9 GB yaklaşık 7 saniye).

### Kurulum

Windows 10/11, x64 ve ARM64. Yönetici yetkisi gerekmez.

```powershell
iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/install.ps1 | iex
```

`%LOCALAPPDATA%\Programs\kazma` klasörüne kurar ve `kz` komutunu PATH'e ekler. Diğer yollar:

- **Scoop**: `scoop bucket add kazma https://github.com/hhasanguclu/kazma`, ardından `scoop install kazma`
- **winget**: her release için manifestler [`packaging/winget`](packaging/winget) içinde üretilir; kabul edilince `winget install hhasanguclu.kazma`
- **Elle**: [son release](https://github.com/hhasanguclu/kazma/releases/latest) sayfasından `kz.exe`'yi indirin

> Exe henüz kod imzalı değil; SmartScreen uyarırsa **Ek bilgi -> Yine de çalıştır** deyin.
> SignPath ile imzalama hazır, ayrıntılar [docs/SIGNING.md](docs/SIGNING.md) içinde.

**Kaldırma**: `kz remove` (exe'yi, PATH kaydını ve `%LOCALAPPDATA%\kazma` klasörünü kaldırır).

### Komutlar

| Komut | Ne yapar |
|---|---|
| `kz` | Etkileşimli menü |
| `kz clean` | Geçici dosyalar, çökme dökümleri, tarayıcı / uygulama / geliştirici önbellekleri, Geri Dönüşüm Kutusu |
| `kz deepclean` | Windows.old, yükseltme kurulum dosyaları, ESD dosyaları, bellek dökümleri, hazırda bekletme dosyası, eski geri yükleme noktaları (yönetici) |
| `kz analyze [yol]` | Disk gezgini: `t` ağaç haritası, `b` en büyük dosyalar, `s` sıralama, `/` arama, `w` kim kullanıyor; özel dosyaları açıklar (hiberfil, WSL diskleri, iPhone yedekleri...); silinenler Geri Dönüşüm Kutusu'na gider |
| `kz types [yol]` | Hangi tür dosyalar yer kaplıyor (video, resim, arşiv...) ve en büyük uzantılar |
| `kz purge [yollar]` | Derleme çıktıları: `node_modules`, `target`, `bin/obj`, `.venv`, `__pycache__`... |
| `kz installer [yollar]` | İndirilenler ve Masaüstü'ndeki eski `.exe`, `.msi`, `.msix`, `.iso` dosyaları |
| `kz dupes [yollar]` | Yinelenen dosyalar (boyut -> kısmi özet -> BLAKE3, hepsi paralel) |
| `kz empty [yollar]` | Boş klasörler, bozuk kısayollar ve 0 baytlık dosyalar (Geri Dönüşüm Kutusu'na) |
| `kz privacy` | Son dosyalar, Çalıştır / Gezgin geçmişi, pano, tarayıcı geçmişi ve çerezler (isteğe bağlı) |
| `kz uninstall` | Klasik, Microsoft Store ve Scoop uygulamalarını kaldırır; ardından kalıntı klasörleri ve kayıt defteri anahtarlarını (önce yedekleyerek) temizler; `--leftovers` başka yolla silinmiş uygulamaların kalıntılarını bulur |
| `kz upgrade` | Yüklü programları winget ile günceller |
| `kz drivers` | Windows Update'ten sürücü güncellemeleri; `--installed` üçüncü taraf sürücüleri yaşına göre listeler |
| `kz startup` | Başlangıç programlarını açar / kapatır (Görev Yöneticisi ile aynı ayar, geri alınabilir) |
| `kz debloat` | Sabit bir listedeki tanıtım uygulamalarını kaldırır, reklamları kapatır; `--restore` geri alır |
| `kz backup [klasör]` | Yeni bilgisayar paketi: winget program listesi, sürücüler, program listesi, kazma ayarları; `kz backup restore KLASÖR` geri kurar |
| `kz health` | Disk sağlığı, pil yıpranması, mavi ekranlar, hatalar, güncellemeler, Defender, güvenlik duvarı |
| `kz status` | Canlı CPU, GPU, bellek, disk, ağ, pil; `enter` sürecin neden çalıştığını açıklar, `x` sonlandırır, `m` belleğe göre sıralar |
| `kz power` | Güç planı (`set saver / balanced / high / ultimate`), `battery` raporu, `wake` uyanma nedenleri, `hibernate off / reduced / full` |
| `kz restore` | Geri yükleme noktaları: `list`, `create "açıklama"`, `enable`, `open` (Windows'un kendi sihirbazı) |
| `kz report` | HTML sistem raporu: donanım, sağlık, başlangıç, programlar, sürücüler |
| `kz bench [yol]` | Disk hız testi: sıralı ve 4K rastgele, işletim sistemi önbelleği devre dışı |
| `kz net` | IP ve gecikme; `dns` karşılaştırma, `dns set cloudflare`; `speed` hız testi; `wifi [--show]` |
| `kz ports [filtre]` | Hangi port hangi programda; `--kill PORT` programı kapatır |
| `kz optimize` | Bozuk başlangıç kayıtları (yedekleyerek), Başlat/Arama yeniden başlatma, DNS, ikon önbelleği, zamanı gelince SSD TRIM / DISM / SFC |
| `kz repair [düzeltme]` | `network`, `update` (Windows Update sıfırlama), `store`, `dism` (+SFC), `time`, `printer`; önce geri yükleme noktası (yönetici) |
| `kz schedule weekly` | `kz clean --yes`'i otomatik çalıştırır (`daily`, `weekly`, `monthly`, `off`, `status`) |
| `kz lock <yol>` | Bir dosyayı ya da klasörü hangi programın açık tuttuğu |
| `kz awake [dakika]` | Ekranı ve bilgisayarı uyanık tutar |
| `kz cleanscreen [saniye]` | Ekranı silmek için girdileri yok sayan siyah ekran |
| `kz whitelist` | kazma'nın asla silmeyeceği yollar (`--add`, `--remove`, `--edit`) |
| `kz rules` | `kz clean` için kendi temizlik kuralların (`--edit` Not Defteri'nde açar) |
| `kz context install` | Gezgin sağ tık menüsüne "kazma ile analiz et" ekler (`uninstall` kaldırır) |
| `kz config` | `kz config lang en`, `kz config theme amber` (green, amber, white, blue) |
| `kz update` | kazma'yı GitHub'dan günceller (sağlama toplamı doğrulanır) |
| `kz history` | kazma'nın yaptığı her şeyin kaydı |
| `kz remove` | kazma'yı kaldırır |

Silen ya da kuran her komutta önce `--dry-run` (`-n`) ile önizleme yapabilirsiniz.
Sürücü kurmak, yönetici gerektiren bakım işlerini yapmak ve Windows Update / sistem geçici
dosyalarını temizlemek için terminali **yönetici olarak** açın.

### Güvenlik

- `clean` yalnızca bilinen, yeniden oluşturulabilen önbellek ve geçici konumlara dokunur; 24 saatten yeni geçici dosyalar kalır.
- `purge` bir klasörü yalnızca yanında proje dosyası varsa siler (`package.json`, `Cargo.toml`, `.csproj`...).
- Sürücü kökleri, kullanıcı klasörü, `Windows`, `Program Files`, `AppData` gibi klasörler kesinlikle korunur.
- `analyze`, `installer`, `dupes` ve `uninstall` kalıntıları **Geri Dönüşüm Kutusu**'na gider.
- `dupes` her gruptan en az bir dosyayı her zaman tutar.
- `drivers` yalnızca **Windows Update**'in sunduğu sürücüleri kurar ve önce **geri yükleme noktası** oluşturur.
- `startup` yalnızca Görev Yöneticisi'nin açık / kapalı ayarını değiştirir; `debloat` yalnızca listedeki uygulamaları kaldırır, ayarlarını `--restore` geri alır.
- `update` indirilen dosyayı release'teki `SHA256SUMS.txt` ile doğrular.
- `deepclean`, Windows.old ve yükseltme dosyalarını Windows'un kendi Disk Temizleme aracıyla kaldırır; en yeni geri yükleme noktası her zaman kalır.
- `repair`, Windows Update klasörlerini silmez, yeniden adlandırır; önce geri yükleme noktası önerir.
- MFT okuyucu diski yalnızca okur, hiçbir şey yazmaz.
- Kayıt defteri kalıntıları ve bozuk başlangıç kayıtları silinmeden önce `%LOCALAPPDATA%\kazma\backups` klasörüne yedeklenir.
- `uninstall --leftovers` önerileri seçili gelmez ve Geri Dönüşüm Kutusu'na gider.
- **Whitelist**: `%LOCALAPPDATA%\kazma\whitelist.txt` içindeki yollar asla silinmez. Dosya var ama okunamıyorsa kazma çalışmayı reddeder.
- Her işlem kaydedilir (`kz history`); terminal yoksa `--yes` verilmeden hiçbir şey silinmez.

### Derleme

```bash
cargo build --release                                  # Windows'ta -> target\release\kz.exe
rustup target add x86_64-pc-windows-gnu                # Linux'tan çapraz derleme
sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
```

Release çıkarmak için `Cargo.toml`'daki `version`'ı artırıp `main`'e push etmeniz yeterli: CI Windows'ta
test eder, salt-okunur komutların hepsini gerçekten çalıştırır, sonra `vX.Y.Z` release'ini `kz.exe`, zip ve
`SHA256SUMS.txt` ile yayınlar ve Scoop / winget manifestlerini günceller.

---

## Screenshots / Ekran görüntüleri

| | |
|---|---|
| **Main menu, Turkish** / Ana menü, Türkçe<br><img src="docs/screenshots/menu-tr.png" alt="main menu, Turkish"> | **Main menu, English, amber theme** / Ana menü, İngilizce, amber tema<br><img src="docs/screenshots/menu-en.png" alt="main menu, English, amber theme"> |
| **`kz analyze`** / disk gezgini<br><img src="docs/screenshots/analyze-tr.png" alt="analyze"> | **`kz analyze` + `b`**: largest files / en büyük dosyalar<br><img src="docs/screenshots/analyze-largest.png" alt="largest files"> |
| **`kz analyze` + `t`**: treemap / ağaç haritası<br><img src="docs/screenshots/treemap-tr.png" alt="treemap"> | **`kz types`**: file types / dosya türleri<br><img src="docs/screenshots/types-en.png" alt="file types"> |
| **`kz clean`** selection / seçim ekranı<br><img src="docs/screenshots/clean-tr.png" alt="clean"> | **`kz purge`**, blue theme / mavi tema<br><img src="docs/screenshots/purge-en.png" alt="purge"> |
| **`kz health`** on Windows / Windows'ta<br><img src="docs/screenshots/health-windows.png" alt="health report on Windows"> | **`kz startup --list`** on Windows / Windows'ta<br><img src="docs/screenshots/startup-windows.png" alt="startup list on Windows"> |

<sub>`health` and `startup` were captured from a real Windows run (GitHub Actions, Windows Server 2025).
The other screens come from the same code running in a terminal with demo data, which is why their paths
look like `/home/user/demo`. / `health` ve `startup` gerçek bir Windows çalıştırmasından (GitHub Actions,
Windows Server 2025) alındı. Diğerleri aynı kodun demo verilerle terminalde çalıştırılmasından alındı;
yollar bu yüzden `/home/user/demo` şeklinde görünüyor.</sub>

---

## Credits / Teşekkürler

**Inspiration / İlham.** kazma was inspired by [Mole](https://github.com/tw93/Mole) by
[tw93](https://github.com/tw93), a deep-clean toolkit for macOS. kazma is written from scratch in Rust and
shares no code with it. / kazma, macOS için geliştirilmiş [Mole](https://github.com/tw93/Mole)'dan
esinlendi; sıfırdan Rust ile yazıldı, onunla ortak kodu yoktur.

**Libraries / Kütüphaneler.** Thanks to the authors and maintainers of / Şu projelerin yazarlarına ve
bakımcılarına teşekkürler:

| Crate | Used for / Kullanım | License |
|---|---|---|
| [ratatui](https://github.com/ratatui/ratatui) | terminal UI / terminal arayüzü | MIT |
| [crossterm](https://github.com/crossterm-rs/crossterm) | terminal input & colors / klavye ve renkler | MIT |
| [clap](https://github.com/clap-rs/clap) | command line / komut satırı | MIT / Apache-2.0 |
| [sysinfo](https://github.com/GuillaumeGomez/sysinfo) | CPU, memory, disks, processes / sistem bilgisi | MIT |
| [rayon](https://github.com/rayon-rs/rayon) | parallel scanning / paralel tarama | MIT / Apache-2.0 |
| [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) | duplicate hashing / yinelenen dosya özeti | CC0 / Apache-2.0 |
| [trash](https://github.com/Byron/trash-rs) | Recycle Bin / Geri Dönüşüm Kutusu | MIT |
| [serde](https://github.com/serde-rs/serde), [serde_json](https://github.com/serde-rs/json) | PowerShell JSON | MIT / Apache-2.0 |
| [anyhow](https://github.com/dtolnay/anyhow) | errors / hata yönetimi | MIT / Apache-2.0 |
| [windows-sys](https://github.com/microsoft/windows-rs) | Windows API | MIT / Apache-2.0 |
| [winresource](https://github.com/BenjaminRi/winresource) | exe icon & version info / exe ikonu ve sürüm bilgisi | MIT |

**Platform.** Microsoft's [winget](https://github.com/microsoft/winget-cli) (program updates), the Windows Update
Agent API (driver updates), [Scoop](https://scoop.sh) (distribution) and the
[SignPath Foundation](https://signpath.org) (free code signing for open source).

**Art.** The banner uses the classic [FIGlet](http://www.figlet.org) "standard" letter style; screenshots
are rendered with the [DejaVu Sans Mono](https://dejavu-fonts.github.io) font. / Afiş klasik FIGlet "standard"
harf stilinde; ekran görüntüleri DejaVu Sans Mono yazı tipiyle çizildi.

**Author / Yazar.** [hhasanguclu](https://github.com/hhasanguclu). Bug reports, ideas and pull requests are
welcome / Hata bildirimleri, fikirler ve pull request'ler memnuniyetle karşılanır.

## License / Lisans

[MIT](LICENSE)
