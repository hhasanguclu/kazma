<div align="center">

```
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
```

**Windows için oldschool, tamamen ASCII sistem araç kutusu**

[![release](https://img.shields.io/github/v/release/hhasanguclu/kazma?style=flat-square)](https://github.com/hhasanguclu/kazma/releases/latest)
[![ci](https://img.shields.io/github/actions/workflow/status/hhasanguclu/kazma/ci.yml?branch=main&style=flat-square&label=windows%20ci)](https://github.com/hhasanguclu/kazma/actions)
[![license](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
![platform](https://img.shields.io/badge/windows-10%20%7C%2011-blue?style=flat-square)

[English](README.md) · **Türkçe** · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [Polski](README.pl.md) · [Українська](README.uk.md) · [Русский](README.ru.md)

<img src="docs/screenshots/menu-en.png" width="49%" alt="kazma main menu in English"> <img src="docs/screenshots/menu-ja.png" width="49%" alt="kazma main menu in Japanese, blue theme">

</div>

---

**kazma**, Windows bilgisayarı terminalden temizleyen, inceleyen ve bakımını yapan tek bir
`kz.exe` dosyasıdır (~5 MB). Görünümü eski yeşil ekranlı terminaller gibidir. Kurulum gerektirmez,
arka planda servis çalıştırmaz, veri toplamaz. Arayüz 11 dilde kullanılabilir (English, Türkçe, 简体中文, 繁體中文, 日本語, 한국어, Deutsch, Français, Polski,
Українська, Русский) ve Windows'un görüntü dilinde açılır; `kz config` ekranından ok tuşlarıyla ya da `kz config lang en` ile değiştirebilirsiniz.

## Özellikler

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

## Kurulum

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

## Komutlar

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
| `kz config` | Dili ve temayı ok tuşlarıyla seçin; ya da `kz config lang en` (auto, en, tr, zh-CN, zh-TW, ja, ko, de, fr, pl, uk, ru), `kz config theme amber` (green, amber, white, blue) |
| `kz update` | kazma'yı GitHub'dan günceller (sağlama toplamı doğrulanır) |
| `kz history` | kazma'nın yaptığı her şeyin kaydı |
| `kz remove` | kazma'yı kaldırır |

Silen ya da kuran her komutta önce `--dry-run` (`-n`) ile önizleme yapabilirsiniz.
Sürücü kurmak, yönetici gerektiren bakım işlerini yapmak ve Windows Update / sistem geçici
dosyalarını temizlemek için terminali **yönetici olarak** açın.

## Güvenlik

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

## Derleme

```bash
cargo build --release                                  # Windows'ta -> target\release\kz.exe
rustup target add x86_64-pc-windows-gnu                # Linux'tan çapraz derleme
sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
```

Release çıkarmak için `Cargo.toml`'daki `version`'ı artırıp `main`'e push etmeniz yeterli: CI Windows'ta
test eder, salt-okunur komutların hepsini gerçekten çalıştırır, sonra `vX.Y.Z` release'ini `kz.exe`, zip ve
`SHA256SUMS.txt` ile yayınlar ve Scoop / winget manifestlerini günceller.

## Ekran görüntüleri

| | |
|---|---|
| **`kz`** · English<br><img src="docs/screenshots/menu-en.png" alt="kz main menu, English"> | **`kz`** · 日本語<br><img src="docs/screenshots/menu-ja.png" alt="kz main menu, Japanese, blue theme"> |
| **`kz`** · Türkçe<br><img src="docs/screenshots/menu-tr.png" alt="kz main menu, Turkish, amber theme"> | **`kz analyze`** · `(i)`<br><img src="docs/screenshots/analyze-tr.png" alt="kz analyze"> |
| **`kz analyze`** + `b`<br><img src="docs/screenshots/analyze-largest.png" alt="kz analyze, largest files"> | **`kz analyze`** + `t`<br><img src="docs/screenshots/treemap-tr.png" alt="kz analyze, treemap"> |
| **`kz clean`** · Türkçe<br><img src="docs/screenshots/clean-tr.png" alt="kz clean, Turkish"> | **`kz clean`** · Deutsch<br><img src="docs/screenshots/clean-de.png" alt="kz clean, German, white theme"> |
| **`kz types`**<br><img src="docs/screenshots/types-en.png" alt="kz types"> | **`kz purge`**<br><img src="docs/screenshots/purge-en.png" alt="kz purge, blue theme"> |
| **`kz health`** · Windows<br><img src="docs/screenshots/health-windows.png" alt="kz health on Windows"> | **`kz health`** · Windows · 日本語<br><img src="docs/screenshots/health-windows-ja.png" alt="kz health on Windows, Japanese"> |
| **`kz startup --list`** · Windows<br><img src="docs/screenshots/startup-windows.png" alt="kz startup on Windows"> |  |

<sub>`health` ve `startup` gerçek bir Windows çalıştırmasından (GitHub Actions, Windows Server 2025) alındı.
Diğerleri aynı kodun demo verilerle terminalde çalıştırılmasından alındı; yollar bu yüzden `/home/user/demo`
şeklinde görünüyor.</sub>

## Teşekkürler

**İlham.** kazma, [tw93](https://github.com/tw93) tarafından macOS için geliştirilen [Mole](https://github.com/tw93/Mole)'dan
esinlendi; sıfırdan Rust ile yazıldı, onunla ortak kodu yoktur.

**Çeviriler.** İngilizce ve Türkçe dışındaki diller [`locales/`](locales) klasöründe, İngilizce metni çeviriyle eşleyen
JSON dosyalarıdır. Düzeltmeler memnuniyetle karşılanır: dosyayı düzenleyip pull request açın
(`python tools/i18n_extract.py --missing de` bir dilde eksik kalanları listeler).

**Kütüphaneler.** Şu projelerin yazarlarına ve bakımcılarına teşekkürler: [ratatui](https://github.com/ratatui/ratatui),
[crossterm](https://github.com/crossterm-rs/crossterm), [clap](https://github.com/clap-rs/clap),
[sysinfo](https://github.com/GuillaumeGomez/sysinfo), [rayon](https://github.com/rayon-rs/rayon),
[BLAKE3](https://github.com/BLAKE3-team/BLAKE3), [trash](https://github.com/Byron/trash-rs),
[serde](https://github.com/serde-rs/serde), [anyhow](https://github.com/dtolnay/anyhow),
[syn](https://github.com/dtolnay/syn) / [quote](https://github.com/dtolnay/quote),
[windows-sys](https://github.com/microsoft/windows-rs), [winresource](https://github.com/BenjaminRi/winresource).
Lisanslar için [İngilizce README](README.md#credits)'ye bakın.

**Platform.** Microsoft [winget](https://github.com/microsoft/winget-cli) (program güncellemeleri), Windows Update
Agent API (sürücü güncellemeleri), [Scoop](https://scoop.sh) (dağıtım) ve [SignPath Foundation](https://signpath.org)
(açık kaynak için ücretsiz kod imzalama).

**Görsel.** Afiş klasik [FIGlet](http://www.figlet.org) "standard" harf stilinde; ekran görüntüleri
[DejaVu Sans Mono](https://dejavu-fonts.github.io) yazı tipiyle çizildi.

**Yazar.** [hhasanguclu](https://github.com/hhasanguclu). Hata bildirimleri, fikirler ve pull request'ler memnuniyetle karşılanır.

## Lisans

[MIT](LICENSE)
