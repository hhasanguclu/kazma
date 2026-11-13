<div align="center">

```
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
```

**老派、全 ASCII 的 Windows 系統工具箱**

[![release](https://img.shields.io/github/v/release/hhasanguclu/kazma?style=flat-square)](https://github.com/hhasanguclu/kazma/releases/latest)
[![ci](https://img.shields.io/github/actions/workflow/status/hhasanguclu/kazma/ci.yml?branch=main&style=flat-square&label=windows%20ci)](https://github.com/hhasanguclu/kazma/actions)
[![license](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
![platform](https://img.shields.io/badge/windows-10%20%7C%2011-blue?style=flat-square)

[English](README.md) · [Türkçe](README.tr.md) · [简体中文](README.zh-CN.md) · **繁體中文** · [日本語](README.ja.md) · [한국어](README.ko.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [Polski](README.pl.md) · [Українська](README.uk.md) · [Русский](README.ru.md)

<img src="docs/screenshots/menu-en.png" width="49%" alt="kazma main menu in English"> <img src="docs/screenshots/menu-ja.png" width="49%" alt="kazma main menu in Japanese, blue theme">

</div>

---

**kazma**（土耳其文的「十字鎬」）是一個約 2.7 MB 的單一 `kz.exe`，讓你在終端機裡清理、探索與維護
Windows 電腦，帶著復古的綠色螢幕風格。不需安裝程式、沒有背景服務、沒有遙測。
介面支援 11 種語言（English、Türkçe、简体中文、繁體中文、日本語、한국어、Deutsch、Français、Polski、
Українська、Русский），並會以你的 Windows 顯示語言啟動；可用 `kz config lang de` 變更。

## 功能

| | |
|---|---|
| **清理** | 100 多種快取、暫存檔、記錄檔與資源回收筒，包括 AI 工具、遊戲啟動器與顯示卡驅動程式殘留檔案（`clean`）· Windows.old、升級檔、記憶體傾印、休眠檔、舊的還原點（`deepclean`）· 附樹狀圖與最大檔案檢視的磁碟探索器（`analyze`）· 檔案類型（`types`）· 專案建置產物（`purge`）· 舊安裝檔（`installer`）· 重複檔案（`dupes`）· 空資料夾與失效捷徑（`empty`）· 使用痕跡（`privacy`） |
| **程式與驅動程式** | 解除安裝程式及其殘留檔案（`uninstall`）· 用 winget 更新程式（`upgrade`）· 從 Windows Update 更新驅動程式（`drivers`）· 啟動應用程式（`startup`）· 推廣應用程式與 Windows 廣告（`debloat`）· 搬到新電腦的備份（`backup`） |
| **系統** | 健康報告（`health`）· 含 GPU 與電池的即時監控，以及會說明處理程序為何在執行的處理程序管理員（`status`）· 維護：DNS、圖示快取、SSD TRIM、DISM、SFC（`optimize`）· 修復：網路、Windows Update、Store、系統檔案（`repair`）· 電源計劃與電池（`power`）· 還原點（`restore`）· HTML 報告（`report`）· 磁碟速度測試（`bench`）· 自動清理（`schedule`） |
| **網路** | IP、延遲、DNS 效能比較與切換、測速、已儲存的 Wi-Fi（`net`）· 哪個程式使用哪個連接埠（`ports`） |
| **實用工具** | 哪個程式開著某個檔案（`lock`）· 讓螢幕保持開啟（`awake`）· 擦拭螢幕模式（`cleanscreen`） |
| **kazma** | 受保護的路徑（`whitelist`）· 你自己的清理規則（`rules`）· 檔案總管右鍵選單（`context`）· 語言與主題（`config`）· 自我更新（`update`）· 記錄（`history`）· 解除安裝 kazma（`remove`） |

每個清單都支援 `/` 搜尋，也能用滑鼠操作（按一下選取、再按一下開啟、滾輪捲動）。
有新版本推出時，選單會通知你。

它很快。以系統管理員身分在 NTFS 磁碟機上執行時，`analyze` 和 `types` 會直接讀取磁碟機的主檔案表（MFT，
也就是 WizTree 背後的技巧），整個磁碟機的大小幾秒內就能算好。否則會以平行方式計算大小；`dupes` 先比對大小，
再比對開頭與結尾的 64 KB，最後才用 BLAKE3 雜湊整個檔案（83,000 個檔案 / 5.9 GB 約 7 秒）。

## 安裝

Windows 10/11，x64 與 ARM64。不需要系統管理員權限。

```powershell
iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/install.ps1 | iex
```

這會安裝到 `%LOCALAPPDATA%\Programs\kazma`，並把 `kz` 加入你的 PATH。其他方式：

- **Scoop**：`scoop bucket add kazma https://github.com/hhasanguclu/kazma`，然後 `scoop install kazma`
- **winget**：每個發行版本的資訊清單都會產生在 [`packaging/winget`](packaging/winget)；審核通過後：`winget install hhasanguclu.kazma`
- **手動**：從[最新發行版本](https://github.com/hhasanguclu/kazma/releases/latest)下載 `kz.exe`

> 這個 exe 目前尚未進行程式碼簽署，因此 SmartScreen 可能會發出警告：**其他資訊 -> 仍要執行**。
> 透過 SignPath 簽署的準備工作已完成，詳見 [docs/SIGNING.md](docs/SIGNING.md)。

**解除安裝**：`kz remove`（移除 exe、其 PATH 項目以及 `%LOCALAPPDATA%\kazma`）。

## 命令

| 命令 | 用途 |
|---|---|
| `kz` | 互動式選單 |
| `kz clean` | 暫存檔、當機傾印、瀏覽器 / 應用程式 / 開發工具快取、資源回收筒 |
| `kz deepclean` | Windows.old、升級安裝檔、ESD 檔案、記憶體傾印、休眠檔、較舊的還原點（需系統管理員） |
| `kz analyze [path]` | 磁碟使用量探索器：`t` 樹狀圖、`b` 最大檔案、`s` 排序、`/` 搜尋、`w` 誰在使用；會說明特殊檔案（hiberfil、WSL 磁碟、iPhone 備份...）；刪除的項目會移至資源回收筒 |
| `kz types [path]` | 哪類檔案佔用空間（影片、圖片、封存檔...）以及前幾名副檔名 |
| `kz purge [paths...]` | 建置產物：`node_modules`、`target`、`bin/obj`、`.venv`、`__pycache__`、... |
| `kz installer [paths...]` | 下載與桌面中的舊安裝檔（`.exe`、`.msi`、`.msix`、`.iso`） |
| `kz dupes [paths...]` | 重複檔案（大小 -> 部分雜湊 -> BLAKE3，全部平行處理） |
| `kz empty [paths...]` | 空資料夾、失效捷徑與 0 位元組檔案（移至資源回收筒） |
| `kz privacy` | 最近的檔案、執行 / 檔案總管記錄、剪貼簿、瀏覽器記錄與 Cookie（需自行勾選） |
| `kz uninstall` | 移除傳統程式、Microsoft Store 與 Scoop 應用程式，再清除殘留的資料夾與登錄機碼（先備份）；`--leftovers` 可找出以其他方式移除的應用程式殘留 |
| `kz upgrade` | 透過 winget 更新已安裝的程式 |
| `kz drivers` | 從 Windows Update 更新驅動程式；`--installed` 依年份列出第三方驅動程式 |
| `kz startup` | 開啟/關閉啟動應用程式（與工作管理員使用相同旗標，完全可復原） |
| `kz debloat` | 依固定清單移除預先安裝的推廣應用程式，並關閉廣告；`--restore` 可復原 |
| `kz backup [folder]` | 新電腦套件：winget 應用程式清單、驅動程式、程式清單、kazma 設定；`kz backup restore FOLDER` 重新安裝 |
| `kz health` | 磁碟 SMART、電池耗損、藍色畫面、錯誤、更新、Defender、防火牆 |
| `kz status` | 即時 CPU、GPU、記憶體、磁碟、網路、電池；`enter` 說明處理程序為何在執行，`x` 結束它，`m` 依記憶體排序 |
| `kz power` | 電源計劃（`set saver / balanced / high / ultimate`）、`battery` 報告、`wake` 喚醒來源、`hibernate off / reduced / full` |
| `kz restore` | 系統還原點：`list`、`create "desc"`、`enable`、`open`（Windows 內建精靈） |
| `kz report` | HTML 系統報告：硬體、健康狀態、啟動項目、程式、驅動程式 |
| `kz bench [path]` | 磁碟速度測試：循序與 4K 隨機，略過作業系統快取 |
| `kz net` | IP 與延遲；`dns` 效能比較、`dns set cloudflare`；`speed` 測速；`wifi [--show]` |
| `kz ports [filter]` | 哪個程式在接聽哪個連接埠；`--kill PORT` 將它關閉 |
| `kz optimize` | 失效的啟動項目（先備份）、重新啟動開始功能表/搜尋、清除 DNS、圖示快取，需要時執行 SSD TRIM / DISM / SFC |
| `kz repair [fix]` | `network`、`update`（重設 Windows Update）、`store`、`dism`（+SFC）、`time`、`printer`；會先建立還原點（需系統管理員） |
| `kz schedule weekly` | 自動執行 `kz clean --yes`（`daily`、`weekly`、`monthly`、`off`、`status`） |
| `kz lock <path>` | 哪個程式開著某個檔案或資料夾 |
| `kz awake [minutes]` | 讓螢幕與電腦保持喚醒 |
| `kz cleanscreen [seconds]` | 忽略輸入的黑畫面，方便擦拭螢幕 |
| `kz whitelist` | kazma 絕不刪除的路徑（`--add`、`--remove`、`--edit`） |
| `kz rules` | 給 `kz clean` 用的自訂清理規則（`--edit` 會開啟記事本） |
| `kz context install` | 在檔案總管右鍵選單加入「Analyze with kazma」（`uninstall` 可移除） |
| `kz config` | `kz config lang de`（auto、en、tr、zh-CN、zh-TW、ja、ko、de、fr、pl、uk、ru）、`kz config theme amber`（green、amber、white、blue） |
| `kz update` | 從 GitHub 更新 kazma 本身（會驗證總和檢查碼） |
| `kz history` | kazma 所有操作的記錄 |
| `kz remove` | 解除安裝 kazma |

所有會刪除或安裝東西的命令都支援 `--dry-run`（`-n`），可以先預覽。
若要安裝驅動程式、執行需要系統管理員的維護工作，以及清理 Windows Update 與系統暫存快取，
請從**系統管理員**終端機執行（以系統管理員身分執行）。

## 安全性

- `clean` 只處理已知、可重新產生的快取與暫存位置；24 小時內的暫存檔會保留。
- `purge` 只有在資料夾旁有專案標記（`package.json`、`Cargo.toml`、`.csproj`...）時才會刪除該資料夾。
- 磁碟機根目錄、你的使用者設定檔、`Windows`、`Program Files`、`AppData` 及類似資料夾一律封鎖。
- `analyze`、`installer`、`dupes` 和 `uninstall` 的殘留檔案都會移至**資源回收筒**。
- `dupes` 每一組至少會保留一個檔案。
- `drivers` 只安裝 **Windows Update** 提供的驅動程式，並會先建立**還原點**。
- `startup` 只切換工作管理員的開/關旗標；`debloat` 只移除固定清單中的應用程式，`--restore` 可復原其設定。
- `update` 會用發行版本的 `SHA256SUMS.txt` 驗證下載的檔案。
- `deepclean` 透過 Windows 內建的磁碟清理移除 Windows.old 與升級檔，並一律保留最新的還原點。
- `repair` 會將 Windows Update 資料夾改名而非刪除，並先提供建立還原點。
- MFT 讀取器只會讀取磁碟機，絕不寫入。
- 登錄殘留與失效的啟動項目在移除前會先匯出到 `%LOCALAPPDATA%\kazma\backups`。
- `uninstall --leftovers` 的建議項目預設不勾選，並會移至資源回收筒。
- **白名單**：`%LOCALAPPDATA%\kazma\whitelist.txt` 中的路徑絕不會被刪除。若該檔案存在卻無法讀取，kazma 會拒絕執行。
- 每項操作都會記錄（`kz history`）；在沒有終端機的情況下，除非加上 `--yes`，否則不會刪除任何東西。

## 建置

```bash
cargo build --release                                  # on Windows -> target\release\kz.exe
rustup target add x86_64-pc-windows-gnu                # cross-compile from Linux
sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
```

為了方便開發，程式碼也能在 Linux/macOS 上建置；僅限 Windows 的部分以 `cfg(windows)` 隔開。
發行新版時，提高 `Cargo.toml` 中的 `version` 並推送到 `main`：CI 會在 Windows 上測試、實際執行每個唯讀命令，
接著發行附有 `kz.exe`、zip 壓縮檔與 `SHA256SUMS.txt` 的 `vX.Y.Z`，並更新 Scoop / winget 資訊清單。

## 螢幕擷取畫面

| | |
|---|---|
| **`kz`** · English<br><img src="docs/screenshots/menu-en.png" alt="kz main menu, English"> | **`kz`** · 日本語<br><img src="docs/screenshots/menu-ja.png" alt="kz main menu, Japanese, blue theme"> |
| **`kz`** · Türkçe<br><img src="docs/screenshots/menu-tr.png" alt="kz main menu, Turkish, amber theme"> | **`kz analyze`** · `(i)`<br><img src="docs/screenshots/analyze-tr.png" alt="kz analyze"> |
| **`kz analyze`** + `b`<br><img src="docs/screenshots/analyze-largest.png" alt="kz analyze, largest files"> | **`kz analyze`** + `t`<br><img src="docs/screenshots/treemap-tr.png" alt="kz analyze, treemap"> |
| **`kz clean`** · Türkçe<br><img src="docs/screenshots/clean-tr.png" alt="kz clean, Turkish"> | **`kz clean`** · Deutsch<br><img src="docs/screenshots/clean-de.png" alt="kz clean, German, white theme"> |
| **`kz types`**<br><img src="docs/screenshots/types-en.png" alt="kz types"> | **`kz purge`**<br><img src="docs/screenshots/purge-en.png" alt="kz purge, blue theme"> |
| **`kz health`** · Windows<br><img src="docs/screenshots/health-windows.png" alt="kz health on Windows"> | **`kz health`** · Windows · 日本語<br><img src="docs/screenshots/health-windows-ja.png" alt="kz health on Windows, Japanese"> |
| **`kz startup --list`** · Windows<br><img src="docs/screenshots/startup-windows.png" alt="kz startup on Windows"> |  |

<sub>`health` 和 `startup` 是在真實的 Windows 上執行時擷取的（GitHub Actions，Windows Server 2025）。
其他畫面則是同一份程式碼在終端機中以示範資料執行的結果，所以路徑看起來像 `/home/user/demo`。</sub>

## 致謝

**靈感來源。** kazma 的靈感來自 [tw93](https://github.com/tw93) 的 [Mole](https://github.com/tw93/Mole)，
一套 macOS 深度清理工具箱。kazma 以 Rust 從頭撰寫，與它沒有共用任何程式碼。

**翻譯。** 英文與土耳其文以外的每種語言，都以 JSON 檔案存放在 [`locales/`](locales) 中，
將英文文字對應到譯文。非常歡迎指正：編輯該檔案並開一個 pull request 即可
（`python tools/i18n_extract.py --missing de` 會列出某個語言還缺少的內容）。

**程式庫。** 感謝以下專案的作者與維護者：

| Crate | 用途 | 授權 |
|---|---|---|
| [ratatui](https://github.com/ratatui/ratatui) | 終端機 UI | MIT |
| [crossterm](https://github.com/crossterm-rs/crossterm) | 終端機輸入與色彩 | MIT |
| [clap](https://github.com/clap-rs/clap) | 命令列 | MIT / Apache-2.0 |
| [sysinfo](https://github.com/GuillaumeGomez/sysinfo) | CPU、記憶體、磁碟、處理程序 | MIT |
| [rayon](https://github.com/rayon-rs/rayon) | 平行掃描 | MIT / Apache-2.0 |
| [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) | 重複檔案雜湊 | CC0 / Apache-2.0 |
| [trash](https://github.com/Byron/trash-rs) | 資源回收筒 | MIT |
| [serde](https://github.com/serde-rs/serde), [serde_json](https://github.com/serde-rs/json) | PowerShell JSON、翻譯 | MIT / Apache-2.0 |
| [anyhow](https://github.com/dtolnay/anyhow) | 錯誤處理 | MIT / Apache-2.0 |
| [syn](https://github.com/dtolnay/syn), [quote](https://github.com/dtolnay/quote) | `tr!` 翻譯巨集 | MIT / Apache-2.0 |
| [windows-sys](https://github.com/microsoft/windows-rs) | Windows API | MIT / Apache-2.0 |
| [winresource](https://github.com/BenjaminRi/winresource) | exe 圖示與版本資訊 | MIT |

**平台。** Microsoft 的 [winget](https://github.com/microsoft/winget-cli)（程式更新）、Windows Update
Agent API（驅動程式更新）、[Scoop](https://scoop.sh)（散布）以及
[SignPath Foundation](https://signpath.org)（為開放原始碼提供免費程式碼簽署）。

**美術。** 橫幅使用經典的 [FIGlet](http://www.figlet.org)「standard」字型；螢幕擷取畫面
以 [DejaVu Sans Mono](https://dejavu-fonts.github.io) 字型繪製。

**作者。** [hhasanguclu](https://github.com/hhasanguclu)。歡迎回報錯誤、提出想法與發送 pull request。

## 授權

[MIT](LICENSE)
