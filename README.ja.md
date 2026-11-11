<div align="center">

```
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
```

**オールドスクールな、オール ASCII の Windows システム ツールキット**

[![release](https://img.shields.io/github/v/release/hhasanguclu/kazma?style=flat-square)](https://github.com/hhasanguclu/kazma/releases/latest)
[![ci](https://img.shields.io/github/actions/workflow/status/hhasanguclu/kazma/ci.yml?branch=main&style=flat-square&label=windows%20ci)](https://github.com/hhasanguclu/kazma/actions)
[![license](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
![platform](https://img.shields.io/badge/windows-10%20%7C%2011-blue?style=flat-square)

[English](README.md) · [Türkçe](README.tr.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · **日本語** · [한국어](README.ko.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [Polski](README.pl.md) · [Українська](README.uk.md) · [Русский](README.ru.md)

<img src="docs/screenshots/menu-tr.png" width="49%" alt="kazma main menu in Turkish"> <img src="docs/screenshots/menu-en.png" width="49%" alt="kazma main menu in English, amber theme">

</div>

---

**kazma** (トルコ語で「つるはし」) は、約 2.7 MB の `kz.exe` ひとつで Windows PC のクリーンアップ、探索、メンテナンスをターミナルから行えるツールです。見た目はレトロなグリーン スクリーン風。インストーラーも、常駐サービスも、テレメトリもありません。UI は 11 言語 (English, Türkçe, 简体中文, 繁體中文, 日本語, 한국어, Deutsch, Français, Polski,
Українська, Русский) に対応し、Windows の表示言語で起動します。`kz config lang de` で変更できます。

## 機能

| | |
|---|---|
| **クリーンアップ** | 100 種類以上のキャッシュ、一時ファイル、ログ、ごみ箱。AI ツール、ゲーム ランチャー、GPU ドライバーの残りファイルも対象 (`clean`) · Windows.old、アップグレード ファイル、メモリ ダンプ、休止状態ファイル、古い復元ポイント (`deepclean`) · ツリーマップと最大ファイル表示付きのディスク エクスプローラー (`analyze`) · ファイルの種類 (`types`) · プロジェクトのビルド成果物 (`purge`) · 古いインストーラー (`installer`) · 重複ファイル (`dupes`) · 空のフォルダーと壊れたショートカット (`empty`) · 使用履歴 (`privacy`) |
| **プログラムとドライバー** | プログラムとその残りファイルをアンインストール (`uninstall`) · winget でプログラムを更新 (`upgrade`) · Windows Update からのドライバー更新 (`drivers`) · スタートアップ アプリ (`startup`) · 宣伝アプリと Windows の広告 (`debloat`) · 新しい PC への引っ越し用バックアップ (`backup`) |
| **システム** | 正常性レポート (`health`) · GPU とバッテリーも表示するライブ モニター、プロセスが動いている理由を説明するプロセス マネージャー (`status`) · メンテナンス: DNS、アイコン キャッシュ、SSD TRIM、DISM、SFC (`optimize`) · 修復: ネットワーク、Windows Update、Store、システム ファイル (`repair`) · 電源プランとバッテリー (`power`) · 復元ポイント (`restore`) · HTML レポート (`report`) · ディスク速度テスト (`bench`) · 自動クリーンアップ (`schedule`) |
| **ネットワーク** | IP、遅延、DNS ベンチマークと切り替え、速度テスト、保存済み Wi-Fi (`net`) · どのプログラムがどのポートを使っているか (`ports`) |
| **便利ツール** | ファイルを開いたままのプログラムを特定 (`lock`) · 画面をスリープさせない (`awake`) · 画面拭きモード (`cleanscreen`) |
| **kazma** | 保護するパス (`whitelist`) · 独自のクリーンアップ ルール (`rules`) · エクスプローラーの右クリック メニュー (`context`) · 言語とテーマ (`config`) · 自己更新 (`update`) · ログ (`history`) · kazma のアンインストール (`remove`) |

どのリストでも `/` で検索でき、マウスでも操作できます (クリックで選択、もう一度クリックで開く、ホイールでスクロール)。
新しいバージョンが出るとメニューでお知らせします。

とにかく高速です。NTFS ドライブで管理者として実行すると、`analyze` と `types` はドライブのマスター ファイル テーブル (MFT) を直接読み取るため (WizTree と同じ仕組み)、ドライブ全体のサイズが数秒で揃います。それ以外の場合もサイズは並列で計算されます。`dupes` はまずサイズ、次に先頭と末尾の
64 KB を比較し、最後にファイル全体を BLAKE3 でハッシュします (83,000 ファイル / 5.9 GB で約 7 秒)。

## インストール

Windows 10/11、x64 と ARM64。管理者権限は不要です。

```powershell
iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/install.ps1 | iex
```

`%LOCALAPPDATA%\Programs\kazma` にインストールされ、`kz` が PATH に追加されます。ほかの方法:

- **Scoop**: `scoop bucket add kazma https://github.com/hhasanguclu/kazma` の後に `scoop install kazma`
- **winget**: 各リリースのマニフェストは [`packaging/winget`](packaging/winget) に生成されます。承認後は `winget install hhasanguclu.kazma`
- **手動**: [最新リリース](https://github.com/hhasanguclu/kazma/releases/latest) から `kz.exe` をダウンロード

> exe はまだコード署名されていないため、SmartScreen の警告が出ることがあります: **詳細情報 -> 実行**。
> SignPath による署名は準備済みです。[docs/SIGNING.md](docs/SIGNING.md) を参照してください。

**アンインストール**: `kz remove` (exe、PATH の項目、`%LOCALAPPDATA%\kazma` を削除します)。

## コマンド

| コマンド | 内容 |
|---|---|
| `kz` | 対話型メニュー |
| `kz clean` | 一時ファイル、クラッシュ ダンプ、ブラウザー / アプリ / 開発ツールのキャッシュ、ごみ箱 |
| `kz deepclean` | Windows.old、アップグレード用セットアップ ファイル、ESD ファイル、メモリ ダンプ、休止状態ファイル、古い復元ポイント (管理者) |
| `kz analyze [path]` | ディスク使用量エクスプローラー: `t` ツリーマップ、`b` 最大ファイル、`s` 並べ替え、`/` 検索、`w` 使用元。特殊なファイル (hiberfil、WSL ディスク、iPhone バックアップ...) も説明します。削除したものはごみ箱へ |
| `kz types [path]` | 容量を占めているファイルの種類 (ビデオ、画像、アーカイブ...) と上位の拡張子 |
| `kz purge [paths...]` | ビルド成果物: `node_modules`、`target`、`bin/obj`、`.venv`、`__pycache__`、... |
| `kz installer [paths...]` | ダウンロードとデスクトップにある古いセットアップ ファイル (`.exe`、`.msi`、`.msix`、`.iso`) |
| `kz dupes [paths...]` | 重複ファイル (サイズ -> 部分ハッシュ -> BLAKE3、すべて並列) |
| `kz empty [paths...]` | 空のフォルダー、壊れたショートカット、0 バイトのファイル (ごみ箱へ) |
| `kz privacy` | 最近使ったファイル、ファイル名を指定して実行 / エクスプローラーの履歴、クリップボード、ブラウザーの履歴と Cookie (選択制) |
| `kz uninstall` | クラシック アプリ、Microsoft Store アプリ、Scoop アプリを削除し、残ったフォルダーとレジストリ キーも削除 (事前にバックアップ)。`--leftovers` で別の方法で削除したアプリの残骸を検出 |
| `kz upgrade` | winget でインストール済みプログラムを更新 |
| `kz drivers` | Windows Update からのドライバー更新。`--installed` でサードパーティ製ドライバーを古い順に一覧表示 |
| `kz startup` | スタートアップ アプリのオン/オフ (タスク マネージャーと同じフラグ、完全に元に戻せます) |
| `kz debloat` | 固定リストにあるプリインストールの宣伝アプリを削除し、広告をオフ。`--restore` で元に戻す |
| `kz backup [folder]` | 新しい PC 用キット: winget のアプリ一覧、ドライバー、プログラム一覧、kazma の設定。`kz backup restore FOLDER` で再インストール |
| `kz health` | ディスクの SMART、バッテリーの劣化、ブルースクリーン、エラー、更新、Defender、ファイアウォール |
| `kz status` | CPU、GPU、メモリ、ディスク、ネットワーク、バッテリーをリアルタイム表示。`enter` でプロセスが動いている理由を説明、`x` で終了、`m` でメモリ順に並べ替え |
| `kz power` | 電源プラン (`set saver / balanced / high / ultimate`)、`battery` レポート、`wake` スリープ解除の要因、`hibernate off / reduced / full` |
| `kz restore` | システムの復元ポイント: `list`、`create "desc"`、`enable`、`open` (Windows 標準のウィザード) |
| `kz report` | HTML のシステム レポート: ハードウェア、正常性、スタートアップ、プログラム、ドライバー |
| `kz bench [path]` | ディスク速度テスト: シーケンシャルと 4K ランダム、OS キャッシュをバイパス |
| `kz net` | IP と遅延。`dns` ベンチマーク、`dns set cloudflare`、`speed` テスト、`wifi [--show]` |
| `kz ports [filter]` | どのプログラムがどのポートで待ち受けているか。`--kill PORT` で終了 |
| `kz optimize` | 壊れたスタートアップ項目 (バックアップあり)、スタート/検索の再起動、DNS フラッシュ、アイコン キャッシュ、必要に応じて SSD TRIM / DISM / SFC |
| `kz repair [fix]` | `network`、`update` (Windows Update のリセット)、`store`、`dism` (+SFC)、`time`、`printer`。先に復元ポイントを作成 (管理者) |
| `kz schedule weekly` | `kz clean --yes` を自動実行 (`daily`、`weekly`、`monthly`、`off`、`status`) |
| `kz lock <path>` | ファイルやフォルダーを開いているプログラムを表示 |
| `kz awake [minutes]` | 画面と PC をスリープさせない |
| `kz cleanscreen [seconds]` | 入力を無視する黒い画面。ディスプレイを拭くときに |
| `kz whitelist` | kazma が絶対に削除しないパス (`--add`、`--remove`、`--edit`) |
| `kz rules` | `kz clean` 用の独自ルール (`--edit` でメモ帳が開きます) |
| `kz context install` | エクスプローラーの右クリック メニューに "Analyze with kazma" を追加 (`uninstall` で削除) |
| `kz config` | `kz config lang de` (auto, en, tr, zh-CN, zh-TW, ja, ko, de, fr, pl, uk, ru)、`kz config theme amber` (green, amber, white, blue) |
| `kz update` | GitHub から kazma 自体を更新 (チェックサム検証あり) |
| `kz history` | kazma が行ったすべての操作のログ |
| `kz remove` | kazma をアンインストール |

削除やインストールを行うコマンドはすべて `--dry-run` (`-n`) に対応しており、事前に内容を確認できます。
ドライバーのインストール、管理者向けメンテナンス タスクの実行、Windows Update とシステムの一時キャッシュのクリーンアップには、**管理者権限の**ターミナル (管理者として実行) を使ってください。

## 安全性

- `clean` が触るのは、再生成できる既知のキャッシュと一時ファイルの場所だけです。作成から 24 時間未満の一時ファイルは残します。
- `purge` は、プロジェクトの目印 (`package.json`、`Cargo.toml`、`.csproj`...) が隣にあるフォルダーだけを削除します。
- ドライブのルート、ユーザー プロファイル、`Windows`、`Program Files`、`AppData` などのフォルダーは確実にブロックされます。
- `analyze`、`installer`、`dupes`、`uninstall` の残りファイルは**ごみ箱**に移動されます。
- `dupes` は各グループで必ず 1 つ以上のファイルを残します。
- `drivers` は **Windows Update** が提供するドライバーだけをインストールし、先に**復元ポイント**を作成します。
- `startup` はタスク マネージャーのオン/オフ フラグを切り替えるだけです。`debloat` は固定リストにあるアプリだけを削除し、`--restore` で設定を元に戻せます。
- `update` はダウンロードしたファイルをリリースの `SHA256SUMS.txt` で検証します。
- `deepclean` は Windows 標準のディスク クリーンアップを使って Windows.old とアップグレード ファイルを削除し、最新の復元ポイントは必ず残します。
- `repair` は Windows Update のフォルダーを削除せずに名前を変更し、先に復元ポイントの作成を提案します。
- MFT リーダーはドライブを読み取るだけで、書き込むことはありません。
- レジストリの残骸と壊れたスタートアップ項目は、削除前に `%LOCALAPPDATA%\kazma\backups` へエクスポートされます。
- `uninstall --leftovers` の候補は未選択の状態で表示され、削除するとごみ箱に移動されます。
- **ホワイトリスト**: `%LOCALAPPDATA%\kazma\whitelist.txt` にあるパスは決して削除されません。このファイルが存在するのに読み取れない場合、kazma は実行を拒否します。
- すべての操作はログに記録されます (`kz history`)。ターミナルがない場合、`--yes` を指定しない限り何も削除されません。

## ビルド

```bash
cargo build --release                                  # on Windows -> target\release\kz.exe
rustup target add x86_64-pc-windows-gnu                # cross-compile from Linux
sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
```

開発用に Linux/macOS でもビルドできます。Windows 専用の部分は `cfg(windows)` で切り分けています。
リリースするには `Cargo.toml` の `version` を上げて `main` に push します。CI が Windows でテストし、読み取り専用のコマンドをすべて実際に実行してから、`kz.exe`、zip、`SHA256SUMS.txt` を含む `vX.Y.Z` を公開し、
Scoop / winget のマニフェストを更新します。

## スクリーンショット

| | |
|---|---|
| **メイン メニュー (トルコ語)**<br><img src="docs/screenshots/menu-tr.png" alt="main menu, Turkish"> | **メイン メニュー (英語、amber テーマ)**<br><img src="docs/screenshots/menu-en.png" alt="main menu, English, amber theme"> |
| **`kz analyze`**<br><img src="docs/screenshots/analyze-tr.png" alt="analyze"> | **`kz analyze` + `b`**: 最大ファイル<br><img src="docs/screenshots/analyze-largest.png" alt="largest files"> |
| **`kz analyze` + `t`**: ツリーマップ<br><img src="docs/screenshots/treemap-tr.png" alt="treemap"> | **`kz types`**: ファイルの種類<br><img src="docs/screenshots/types-en.png" alt="file types"> |
| **`kz clean`** の選択画面<br><img src="docs/screenshots/clean-tr.png" alt="clean"> | **`kz purge`**、blue テーマ<br><img src="docs/screenshots/purge-en.png" alt="purge"> |
| Windows 上の **`kz health`**<br><img src="docs/screenshots/health-windows.png" alt="health report on Windows"> | Windows 上の **`kz startup --list`**<br><img src="docs/screenshots/startup-windows.png" alt="startup list on Windows"> |

<sub>`health` と `startup` は実際の Windows 上での実行 (GitHub Actions、Windows Server 2025) をキャプチャしたものです。
そのほかの画面は、同じコードをデモ データ付きのターミナルで実行したものなので、パスが
`/home/user/demo` のようになっています。</sub>

## クレジット

**インスピレーション。** kazma は [tw93](https://github.com/tw93) 氏による macOS 向けディープ クリーン ツールキット [Mole](https://github.com/tw93/Mole) に着想を得ています。kazma は Rust でゼロから書かれており、
Mole とコードは共有していません。

**翻訳。** 英語とトルコ語以外の言語はすべて [`locales/`](locales) に、英語のテキストと訳文を対応付けた JSON ファイルとして置かれています。修正は大歓迎です。ファイルを編集してプル リクエストを送ってください
(`python tools/i18n_extract.py --missing de` で、その言語でまだ足りないものを一覧表示できます)。

**ライブラリ。** 以下の作者とメンテナーの皆さんに感謝します:

| クレート | 用途 | ライセンス |
|---|---|---|
| [ratatui](https://github.com/ratatui/ratatui) | ターミナル UI | MIT |
| [crossterm](https://github.com/crossterm-rs/crossterm) | ターミナルの入力と色 | MIT |
| [clap](https://github.com/clap-rs/clap) | コマンド ライン | MIT / Apache-2.0 |
| [sysinfo](https://github.com/GuillaumeGomez/sysinfo) | CPU、メモリ、ディスク、プロセス | MIT |
| [rayon](https://github.com/rayon-rs/rayon) | 並列スキャン | MIT / Apache-2.0 |
| [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) | 重複判定のハッシュ | CC0 / Apache-2.0 |
| [trash](https://github.com/Byron/trash-rs) | ごみ箱 | MIT |
| [serde](https://github.com/serde-rs/serde), [serde_json](https://github.com/serde-rs/json) | PowerShell の JSON、翻訳 | MIT / Apache-2.0 |
| [anyhow](https://github.com/dtolnay/anyhow) | エラー処理 | MIT / Apache-2.0 |
| [syn](https://github.com/dtolnay/syn), [quote](https://github.com/dtolnay/quote) | `tr!` 翻訳マクロ | MIT / Apache-2.0 |
| [windows-sys](https://github.com/microsoft/windows-rs) | Windows API | MIT / Apache-2.0 |
| [winresource](https://github.com/BenjaminRi/winresource) | exe のアイコンとバージョン情報 | MIT |

**プラットフォーム。** Microsoft の [winget](https://github.com/microsoft/winget-cli) (プログラムの更新)、Windows Update
Agent API (ドライバーの更新)、[Scoop](https://scoop.sh) (配布)、そして
[SignPath Foundation](https://signpath.org) (オープンソース向けの無料コード署名)。

**アート。** バナーにはクラシックな [FIGlet](http://www.figlet.org) の "standard" 書体を使っています。スクリーンショットは
[DejaVu Sans Mono](https://dejavu-fonts.github.io) フォントでレンダリングしています。

**作者。** [hhasanguclu](https://github.com/hhasanguclu)。バグ報告、アイデア、プル リクエストを歓迎します。

## ライセンス

[MIT](LICENSE)
